use clap::Parser;
use kvs::ClientCLI;
use kvs::Result;

fn main() -> Result<()> {
    let cli = ClientCLI::parse();
    cli.run()
}
