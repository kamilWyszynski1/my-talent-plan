use crate::{cmd::CMD, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    io::Write,
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
};

/*
  _____ _      _____ ______ _   _ _______
 / ____| |    |_   _|  ____| \ | |__   __|
| |    | |      | | | |__  |  \| |  | |
| |    | |      | | |  __| | . ` |  | |
| |____| |____ _| |_| |____| |\  |  | |
 \_____|______|_____|______|_| \_|  |_|
 */

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct ClientCLI {
    #[clap(subcommand)]
    command: Commands,
}

impl ClientCLI {
    pub fn run(&self) -> Result<()> {
        self.command.run()
    }
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Commands {
    Set {
        key: String,
        value: String,
        #[clap(
            action,
            long,
            default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000), 
            value_parser,
            value_name = "IP-PORT",
        )]
        addr: SocketAddrV4,
    },
    Get {
        key: String,
        #[clap(
            action,
            long,
            default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000), 
            value_parser,
            value_name = "IP-PORT",
        )]
        addr: SocketAddrV4,
    },
    Rm {
        key: String,
        #[clap(
            action,
            long,
            default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000), 
            value_parser,
            value_name = "IP-PORT",
        )]
        addr: SocketAddrV4,
    },
}

impl Commands {
    pub fn run(&self) -> Result<()> {
        let client = Client::new(*self.addr());

        let cmd = match self {
            Commands::Set {
                key,
                value,
                addr: _,
            } => CMD::Set {
                key: key.clone(),
                value: value.clone(),
            },
            Commands::Get { key, addr: _ } => CMD::Get { key: key.clone() },
            Commands::Rm { key, addr: _ } => CMD::Rm { key: key.clone() },
        };

        client.write(&cmd)?;

        Ok(())
    }

    fn addr(&self) -> &SocketAddrV4 {
        match self {
            Commands::Set {
                key: _,
                value: _,
                addr,
            } => addr,
            Commands::Get { key: _, addr } => addr,
            Commands::Rm { key: _, addr } => addr,
        }
    }
}

/// Simple single-threaded tcp client.
struct Client {
    ip: SocketAddrV4,
}

impl Client {
    fn new(ip: SocketAddrV4) -> Self {
        Self { ip }
    }

    fn write(&self, cmd: &CMD) -> Result<()> {
        let mut stream = TcpStream::connect(self.ip)?;

        stream.write_all(&serde_json::to_vec(cmd)?)?;

        Ok(())
    }
}
