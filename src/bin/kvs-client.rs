use clap::Parser;
use kvs::ClientCLI;
use kvs::Result;
use log::LevelFilter;

fn main() -> Result<()> {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let cli = ClientCLI::parse();
    cli.run()
}
