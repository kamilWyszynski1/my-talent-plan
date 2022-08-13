use crate::error::{KvsError, Result};
use crate::reader::{BufReaderWithPos, BufWriterWithPos};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::{collections::HashMap, path::PathBuf};

#[derive(Debug, Serialize, Deserialize)]
enum Command {
    Set { key: String, value: String },
    Rm { key: String },
}

#[derive(Debug)]
/// Represents the position and length of a json-serialized command in the log.d
struct CommandPos {
    pos: u64,
    len: u64,
}

pub struct KvStore {
    path: PathBuf,
    /// Represents command pos and reader for that command.
    data: HashMap<u64, BufReaderWithPos<File>>,
    writer: BufWriterWithPos<File>,

    /// Key is mapped to CommandPos.
    index: BTreeMap<String, CommandPos>,
}

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path: PathBuf = path.into();
        std::fs::create_dir_all(&path)?;
        let path = path.join("db.log");

        debug!("create dir all, path: {:?}", path);

        let mut s = Self {
            path: path.clone(),
            data: HashMap::default(),
            writer: BufWriterWithPos::new(
                OpenOptions::new()
                    .create(true)
                    .write(true)
                    .append(true)
                    .open(&path)?,
            )?,
            index: BTreeMap::default(),
        };

        debug!("writer: {:?}", s.writer);

        let mut reader = BufReader::new(File::open(&path)?);
        let mut pos = reader.seek(SeekFrom::Start(0))?;
        let mut stream = Deserializer::from_reader(reader).into_iter::<Command>();

        debug!("reading stream");
        while let Some(cmd) = stream.next() {
            let new_pos = stream.byte_offset() as u64;

            debug!("cmd: {:?}", cmd);

            match cmd? {
                Command::Set { key, .. } => {
                    s.index.insert(
                        key,
                        CommandPos {
                            pos,
                            len: new_pos - pos,
                        },
                    );
                }
                Command::Rm { key } => {
                    s.index.remove(&key).ok_or(KvsError::KeyNotFound)?;
                }
            }
            pos = new_pos;
        }
        debug!("stream read, index: {:?}", s.index);

        Ok(s)
    }

    pub fn set(&mut self, key: String, value: String) -> Result<()> {
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
                pos,
                len: self.writer.pos - pos,
            },
        );

        Ok(())
    }

    pub fn get(&self, key: String) -> Result<Option<String>> {
        match self.index.get(&key) {
            Some(cmd_pos) => {
                let reader = BufReaderWithPos::new(File::open(&self.path)?, cmd_pos.pos)?;
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

// #[cfg(test)]
// mod tests {
//     use std::{env::current_dir, fs::read_to_string};

//     use super::{KvStore, Result};
//     use anyhow::Ok;
//     use tempfile::TempDir;

//     #[test]

//     fn test_set_and_get() -> Result<()> {
//         env_logger::init();

//         let temp_dir = TempDir::new().expect("unable to create temporary working directory");
//         println!("{:?}", temp_dir.path());
//         let mut store = KvStore::open(temp_dir.path())?;

//         store.set("key1".to_owned(), "value1".to_owned())?;
//         store.set("key2".to_owned(), "value2".to_owned())?;

//         println!("{:?}", read_to_string(temp_dir.path().join("db.log")));

//         println!("{:?}", store.get("key1".to_owned()));
//         Ok(())
//     }
// }
