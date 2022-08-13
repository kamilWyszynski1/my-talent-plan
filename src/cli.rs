use crate::KvStore;
use crate::Result;
use anyhow::Context;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn run(self, kv: &mut KvStore) -> Result<()> {
        self.command.run(kv)
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Set { key: String, value: String },
    Get { key: String },
    Rm { key: String },
}

impl Commands {
    pub fn run(self, kv: &mut KvStore) -> Result<()> {
        match self {
            Commands::Set { key, value } => kv.set(key, value),
            Commands::Get { key } => {
                match kv.get(key)? {
                    Some(v) => println!("{v}"),
                    None => println!("Key not found"),
                }
                // let v = kv.get(key)?.context("value not found")?;
                // println!("{v}");
                Ok(())
            }
            Commands::Rm { key } => kv.remove(key),
        }
    }
}
