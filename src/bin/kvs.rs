use clap::Parser;
use std::env::current_dir;
use std::process;

use kvs::Cli;
use kvs::Result;

fn main() -> Result<()> {
    env_logger::init();

    let mut kvs = kvs::KvStore::open(current_dir()?)?;
    let cli = Cli::parse();
    if let Err(e) = cli.run(&mut kvs) {
        println!("{}", e);
        process::exit(1);
    }
    Ok(())
}
