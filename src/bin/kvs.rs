use clap::Parser;

use kvs::Cli;

fn main() {
    let mut kvs = kvs::KvStore::new();
    let cli = Cli::parse();
    cli.run(&mut kvs);
}
