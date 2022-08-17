use crate::error::{KvsError, Result};
use crate::reader::{BufReaderWithPos, BufWriterWithPos};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
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

pub struct KvStore {
    path: PathBuf,
    /// Holds [generation:reader] data.
    readers: HashMap<u64, BufReaderWithPos<File>>,
    writer: BufWriterWithPos<File>,

    /// Key is mapped to CommandPos.
    index: BTreeMap<String, CommandPos>,

    /// Current generation.
    generation: u64,

    /// Tracks amount of already written data to latest generation file.
    last_generation_size: u64,
}

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
            readers,
            writer: BufWriterWithPos::new(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(path.join(format!("{}.log", generation)))?,
            )?,
            index: BTreeMap::default(),
            generation,
            last_generation_size: 0, // will be set in read_generation_data method.
        };

        // read all data from all readers.
        s.read_generation_data()?;

        debug!(
            "current generation: {}, last_generation_size: {}",
            s.generation, s.last_generation_size
        );

        Ok(s)
    }

    /// Goes through all existing generation data and reads their content and Command.
    fn read_generation_data(&mut self) -> Result<()> {
        // size of latest generation.
        let mut size = 0;

        for (gen, _) in &self.readers {
            let mut reader = BufReader::new(File::open(self.path.join(format!("{}.log", gen)))?);
            let mut pos = reader.seek(SeekFrom::Start(0))?;
            let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

            while let Some(cmd) = stream.next() {
                let new_pos = stream.byte_offset() as u64;

                debug!("gen:{}, cmd: {:?}", gen, cmd);

                match cmd? {
                    Command::Set { key, .. } => {
                        if *gen == self.generation {
                            size += 1
                        }
                        self.index.insert(
                            key,
                            CommandPos {
                                pos,
                                len: new_pos - pos,
                                gen: *gen,
                            },
                        );
                    }
                    Command::Rm { key } => {
                        self.index.remove(&key).ok_or(KvsError::KeyNotFound)?;
                    }
                }
                pos = new_pos;
            }
        }
        self.last_generation_size = size;
        Ok(())
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        if self.last_generation_size >= CAPACITY {
            self.new_generation()?;
        }

        let pos = self.writer.pos;
        writeln!(
            self.writer,
            "{}",
            serde_json::to_string(&Command::Set {
                key: key.clone(),
                value
            })?
        )?;
        self.writer.flush()?;

        self.index.insert(
            key,
            CommandPos {
                gen: self.generation,
                pos,
                len: self.writer.pos - pos,
            },
        );

        self.last_generation_size += 1;
        Ok(())
    }

    /// Creates new file, new writer and reader for that file and updates `last_generation_size` and `generation` fields.
    pub fn new_generation(&mut self) -> Result<()> {
        debug!("new_generation - current generation: {}", self.generation);

        self.generation += 1;
        self.last_generation_size = 0;

        let new_file_path = self.path.join(format!("{}.log", self.generation));
        self.writer = BufWriterWithPos::new(
            OpenOptions::new()
                .create(true)
                .write(true)
                .append(true)
                .open(&new_file_path)?,
        )?;
        self.readers.insert(
            self.generation,
            BufReaderWithPos::new(File::open(&new_file_path)?, 0)?,
        );

        Ok(())
    }

    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        match self.index.get(&key) {
            Some(cmd_pos) => {
                let reader = self
                    .readers
                    .get_mut(&cmd_pos.gen)
                    .context("could not find a reader")?;

                reader.seek(SeekFrom::Start(cmd_pos.pos))?;
                if let Command::Set { value, .. } =
                    serde_json::from_reader(reader.take(cmd_pos.len))?
                {
                    return Ok(Some(value));
                }
                bail!("Key not found")
            }
            None => Ok(None),
        }
    }

    pub fn remove(&mut self, key: String) -> Result<()> {
        if self.index.remove(&key).is_none() {
            return Err(KvsError::KeyNotFound.into());
        }

        serde_json::to_writer(&mut self.writer, &Command::Rm { key })?;
        self.writer.flush()?;
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
