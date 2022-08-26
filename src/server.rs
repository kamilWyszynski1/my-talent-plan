use super::error::KvsError;
use crate::Result;
use clap::Parser;
use std::io::Read;
use std::net::{Ipv4Addr, TcpListener};
use std::str::FromStr;
use std::thread;
use std::{fmt::Display, net::SocketAddrV4};

/*
   _____ ______ _______      ________ _____
  / ____|  ____|  __ \ \    / /  ____|  __ \
 | (___ | |__  | |__) \ \  / /| |__  | |__) |
  \___ \|  __| |  _  / \ \/ / |  __| |  _  /
  ____) | |____| | \ \  \  /  | |____| | \ \
 |_____/|______|_|  \_\  \/   |______|_|  \_\
*/

#[derive(Debug, Clone, Copy)]
pub enum Engine {
    KVS,
    Sled,
}

impl Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl FromStr for Engine {
    type Err = KvsError;

    fn from_str(s: &str) -> std::result::Result<Engine, KvsError> {
        match s.to_string().to_lowercase().as_str() {
            "kvs" => Ok(Self::KVS),
            "sled" => Ok(Self::Sled),
            _ => Err(KvsError::Parse),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct ServerCLI {
    #[clap(
        action,
        long,
        default_value_t = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 4000), 
        value_parser,
        value_name = "IP-PORT",
    )]
    addr: SocketAddrV4,
    #[clap(
        short,
        long, 
        default_value_t = Engine::KVS,
        value_name = "ENGINE-NAME",
    )]
    engine: Engine,
}

impl ServerCLI {
    pub fn run(&self) -> Result<()> {
        let server = Server::new(self.addr, self.engine);
        server.run()?;
        Ok(())
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Simple single-threader tcp server.
struct Server {
    ip: SocketAddrV4,
    engine_type: Engine,
}

impl Server {
    fn new(ip: SocketAddrV4, engine_type: Engine) -> Self {
        Self { ip, engine_type }
    }

    pub fn run(&self) -> Result<()> {
        info!(
            "version: {}, ip: {}, engine: {}",
            VERSION, self.ip, self.engine_type
        );

        let listener = TcpListener::bind(self.ip)?;
        // accept connections and process them, spawning a new thread for each one
        info!("Server listening on {}", self.ip);

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    println!("New connection: {}", stream.peer_addr()?);

                    let mut buffer = String::new();
                    stream.read_to_string(&mut buffer)?;

                    println!("buffer: {}", buffer);
                    // handle request here.
                }
                Err(e) => {
                    error!("Error: {}", e);
                    /* connection failed */
                }
            }
        }
        Ok(())
    }
}
