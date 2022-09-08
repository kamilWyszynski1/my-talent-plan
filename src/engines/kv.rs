use crate::error::{KvsError, Result};
use crate::reader::{BufReaderWithPos, BufWriterWithPos};
use crate::KvsEngine;
use anyhow::{bail, Context};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex, RwLock};
use std::{collections::HashMap, path::PathBuf};

/// Compaction process will be started after reaching this many entries.
const CAPACITY: u64 = 1000;

#[derive(Debug, Serialize, Deserialize)]
enum Command {
    Set { key: String, value: String },
    Rm { key: String },
}

#[derive(Debug)]
/// Represents the position and length of a json-serialized command in the log.d
struct CommandPos {
    /// We will 'go' to another generation when we reach CAPACITY limit thus
    /// we need to track generation here.
    gen: u64,
    pos: u64,
    len: u64,
}

#[derive(Debug, Clone)]
pub struct KvStore {
    path: PathBuf,
    /// Holds [generation:reader] data.
    readers: Arc<RwLock<HashMap<u64, BufReaderWithPos<File>>>>,
    writer: Arc<RwLock<BufWriterWithPos<File>>>,

    /// Key is mapped to CommandPos.
    index: Arc<RwLock<BTreeMap<String, CommandPos>>>,

    /// Current generation.
    current_gen: Arc<RwLock<u64>>,

    /// Tracks amount of already written data to latest generation file.
    uncompacted: Arc<RwLock<u64>>,
    // Thread pool responsible for multi-threaded functionalities.
    // thread_pool: TP,
}

unsafe impl Send for KvStore {}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();
        std::fs::create_dir_all(&path)?;

        // read already created generation files.
        let mut readers = open_generation_readers(&path)?;
        debug!("readers opened");
        // find latest generation.
        let generation = readers
            .iter()
            .max_by_key(|(k, _)| *k)
            .map(|(k, _)| *k)
            .unwrap_or_default();
        debug!("generation: {}", generation);

        if readers.is_empty() {
            // create first reader if needed.
            debug!(
                "readers are empty, creating first generation, path: {:?}",
                path.join(format!("{}.log", generation))
            );
            readers.insert(
                0,
                BufReaderWithPos::new(
                    OpenOptions::new()
                        .create(true)
                        .read(true)
                        .append(true) // append is used in order to create a file.
                        .open(path.join(format!("{}.log", generation)))?,
                    0,
                )?,
            );
            debug!("first generation created");
        }

        let mut s = Self {
            path: path.clone(),
            readers: Arc::new(RwLock::new(readers)),
            writer: Arc::new(RwLock::new(BufWriterWithPos::new(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path.join(format!("{}.log", generation)))?,
            )?)),
            index: Arc::default(),
            current_gen: Arc::new(RwLock::new(generation)),
            uncompacted: Arc::default(), // will be set in read_generation_data method.
        };

        // read all data from all readers.
        s.read_generation_data()?;

        debug!(
            "current generation: {}, last_generation_size: {:?}",
            s.current_gen.try_read().unwrap(),
            s.uncompacted.try_read().unwrap()
        );

        Ok(s)
    }

    /// Goes through all existing generation data and reads their content and Command.
    fn read_generation_data(&mut self) -> Result<()> {
        // size of latest generation.
        let mut size = 0;

        for gen in self.readers.clone().read().unwrap().keys().sorted() {
            let mut reader = BufReader::new(File::open(self.gen_path(*gen))?);
            let mut pos = reader.seek(SeekFrom::Start(0))?;
            let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

            while let Some(cmd) = stream.next() {
                let new_pos = stream.byte_offset() as u64;

                debug!("gen:{}, cmd: {:?}", gen, cmd);

                match cmd? {
                    Command::Set { key, .. } => {
                        if *gen == self.current_gen.read().unwrap().to_owned() {
                            size += 1
                        }

                        self.index.write().unwrap().insert(
                            key,
                            CommandPos {
                                pos,
                                len: new_pos - pos,
                                gen: *gen,
                            },
                        );
                    }
                    Command::Rm { key } => {
                        self.index
                            .write()
                            .unwrap()
                            .remove(&key)
                            .ok_or(KvsError::KeyNotFound)?;
                    }
                }
                pos = new_pos;
            }
        }
        *self.uncompacted.write().unwrap() = size;
        Ok(())
    }

    fn gen_path(&self, gen: u64) -> PathBuf {
        self.path.join(format!("{}.log", gen))
    }

    /// Takes values from latest generation and writes compacted version of it to another file.
    fn compact(&self) -> Result<()> {
        // increase current gen by 2. current_gen + 1 is for the compaction file.
        let compaction_gen = self.current_gen.read().unwrap().to_owned() + 1;
        *self.current_gen.write().unwrap() += 2;

        *self.writer.write().unwrap() =
            self.new_log_file(self.current_gen.read().unwrap().to_owned())?;

        let mut compaction_writer = self.new_log_file(compaction_gen)?;

        let mut new_pos = 0; // pos in the new log file.

        for cmd_pos in self.index.write().unwrap().values_mut() {
            let mut readers = self.readers.write().unwrap();
            let reader = readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            if reader.pos != cmd_pos.pos {
                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            }

            let mut entry_reader = reader.take(cmd_pos.len);
            let len = io::copy(&mut entry_reader, &mut compaction_writer)?;
            // *cmd_pos = (compaction_gen, new_pos..new_pos + len).into();
            *cmd_pos = CommandPos {
                gen: compaction_gen,
                pos: new_pos,
                len,
            };
            new_pos += len;
        }
        compaction_writer.flush()?;

        // remove stale log files.
        let stale_gens: Vec<_> = self
            .readers
            .clone()
            .read()
            .unwrap()
            .keys()
            .filter(|&&gen| gen < compaction_gen)
            .cloned()
            .collect();
        for stale_gen in stale_gens {
            self.readers.write().unwrap().remove(&stale_gen);
            fs::remove_file(self.gen_path(stale_gen))?;
        }
        *self.uncompacted.write().unwrap() = 0;

        Ok(())
    }

    fn new_log_file(&self, gen: u64) -> Result<BufWriterWithPos<File>> {
        let path = self.gen_path(gen);
        let writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&path)?,
        )?;
        self.readers
            .write()
            .unwrap()
            .insert(gen, BufReaderWithPos::new(File::open(path)?, 0)?);
        Ok(writer)
    }
}

impl KvsEngine for KvStore {
    fn set(&self, key: String, value: String) -> Result<()> {
        if self.uncompacted.read().unwrap().ge(&CAPACITY) {
            self.compact()?;
        }

        let mut writer = self.writer.write().unwrap();
        let mut index = self.index.write().unwrap();
        let pos = writer.pos;

        write!(
            writer,
            "{}",
            serde_json::to_string(&Command::Set {
                key: key.clone(),
                value
            })?
        )?;
        writer.flush()?;

        index.insert(
            key,
            CommandPos {
                gen: self.current_gen.read().unwrap().to_owned(),
                pos,
                len: writer.pos - pos,
            },
        );

        *self.uncompacted.write().unwrap() += 1;
        Ok(())
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        match self.index.clone().read().unwrap().get(&key) {
            Some(cmd_pos) => {
                let mut readers = self.readers.write().unwrap();
                let reader = readers
                    .get_mut(&cmd_pos.gen)
                    .expect("Cannot find log reader");

                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                let mut content = String::new();
                reader.take(cmd_pos.len).read_to_string(&mut content)?;

                if let Command::Set { value, .. } =
                    serde_json::from_str(&content).context(format!(
                        "could not decode from reader, content: {} cmd_pos: {:?}",
                        &content, cmd_pos,
                    ))?
                {
                    return Ok(Some(value));
                }
                bail!("Key not found")
            }
            None => Ok(None),
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        if self.index.write().unwrap().remove(&key).is_none() {
            return Err(KvsError::KeyNotFound.into());
        }

        let mut writer = self.writer.write().unwrap();
        write!(writer, "{}", serde_json::to_string(&Command::Rm { key })?)?;
        writer.flush()?;
        Ok(())
    }
}

fn open_generation_readers(
    path: impl Into<PathBuf>,
) -> Result<HashMap<u64, BufReaderWithPos<File>>> {
    let mut readers = HashMap::default();
    for file in fs::read_dir(path.into())? {
        let file = file?;

        if !file
            .file_name()
            .to_str()
            .context("could not read file name")?
            .contains(".log")
        {
            continue;
        }

        let file_generation: u64 = file
            .file_name()
            .to_str()
            .context("could not read file name")?
            .replace(".log", "")
            .parse()?;
        let f = File::open(file.path())?;

        readers.insert(file_generation, BufReaderWithPos::new(f, 0)?);
    }
    Ok(readers)
}
