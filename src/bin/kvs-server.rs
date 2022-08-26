use clap::Parser;
use kvs::Result;
use kvs::ServerCLI;
use log::LevelFilter;

fn main() -> Result<()> {
    env_logger::builder().filter_level(LevelFilter::Info).init();

    let cli = ServerCLI::parse();
    cli.run()
}
