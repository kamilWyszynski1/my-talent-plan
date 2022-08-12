use clap::{Parser, Subcommand};

use crate::KvStore;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

impl Cli {
    pub fn run(self, kv: &mut KvStore) {
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
    pub fn run(self, kv: &mut KvStore) {
        match self {
            Commands::Set { key, value } => unimplemented!(),
            Commands::Get { key } => unimplemented!(),
            Commands::Rm { key } => unimplemented!(),
        }
    }
}
