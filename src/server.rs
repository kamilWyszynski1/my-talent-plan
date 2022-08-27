use super::error::KvsError;
use crate::cmd::{GetResponse, SetResponse, CMD};
use crate::engines::sled::SledKvsEngine;
use crate::{KvStore, KvsEngine, Result};
use anyhow::bail;
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::fs::OpenOptions;
use std::io::{BufReader, BufWriter, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::str::FromStr;
use std::{fmt::Display, net::SocketAddrV4};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Simple enum for engine changing.
pub enum EngineType {
    Kvs,
    Sled,
}

impl Display for EngineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl FromStr for EngineType {
    type Err = KvsError;

    fn from_str(s: &str) -> std::result::Result<EngineType, KvsError> {
        match s.to_string().to_lowercase().as_str() {
            "kvs" => Ok(Self::Kvs),
            "sled" => Ok(Self::Sled),
            _ => Err(KvsError::Parse),
        }
    }
}

/// Current version of cargo pkg.
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
/// Cli for running server side.
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
        default_value_t = EngineType::Kvs,
        value_name = "ENGINE-NAME",
    )]
    engine: EngineType,
}

impl ServerCLI {
    /// Starts server with given configuration.
    pub fn run(&self) -> Result<()> {
        info!(
            "version: {}, ip: {}, engine: {}",
            VERSION, self.addr, self.engine
        );

        match self.engine {
            EngineType::Kvs => self.run_kvs(),
            EngineType::Sled => self.run_sled(),
        }
    }

    /// Starts server with KvStore as an engine.
    fn run_kvs(&self) -> Result<()> {
        let engine = KvStore::open("kv")?;
        let mut server = Server::new(self.addr, self.engine, engine);
        server.run()?;
        Ok(())
    }

    /// Starts server with SledKvsEngine as an engine.
    fn run_sled(&self) -> Result<()> {
        let engine = SledKvsEngine::new()?;
        let mut server = Server::new(self.addr, self.engine, engine);
        server.run()?;
        Ok(())
    }
}

/// Simple single-threader tcp server.
struct Server<E: KvsEngine> {
    ip: SocketAddrV4,
    engine_type: EngineType,
    engine: E,
}

impl<E: KvsEngine> Server<E> {
    fn new(ip: SocketAddrV4, engine_type: EngineType, engine: E) -> Self {
        Self {
            ip,
            engine_type,
            engine,
        }
    }

    /// Checks "conf" file for determining wether server was already 
    /// started and if so checks if EngineType matches.
    fn verify_conf(&self) -> Result<()> {
        let conf_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open("conf")?;

        let content = std::fs::read_to_string("conf")?;

        if content.is_empty() {
            serde_json::to_writer(BufWriter::new(conf_file), &self.engine_type)?;
            return Ok(());
        }

        let previous_engine_type: EngineType = serde_json::from_str(&content)?;

        if previous_engine_type != self.engine_type {
            bail!("Invalid configuration")
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        self.verify_conf()?;

        let listener = TcpListener::bind(self.ip)?;
        // accept connections and process them, spawning a new thread for each one
        info!("Server listening on {}", self.ip);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.serve(stream) {
                        error!("Error on serving client: {}", e);
                    }
                }
                Err(e) => error!("Connection failed: {}", e),
            }
        }
        Ok(())
    }

    fn serve(&mut self, tcp: TcpStream) -> Result<()> {
        let peer_addr = tcp.peer_addr()?;
        let reader = BufReader::new(&tcp);
        let mut writer = BufWriter::new(&tcp);
        let cmd_reader = Deserializer::from_reader(reader).into_iter::<CMD>();

        for cmd in cmd_reader {
            let cmd = cmd?;

            debug!("Receive request from {}: {:?}", peer_addr, cmd);

            match cmd {
                CMD::Set { key, value } => {
                    debug!("creating response");

                    let response = match self.engine.set(key, value) {
                        Ok(_) => SetResponse::Ok(()),
                        Err(e) => SetResponse::Err(e.to_string()),
                    };

                    debug!("writing response");

                    serde_json::to_writer(&mut writer, &response)?;
                    writer.flush()?;

                    debug!("response written");
                }
                CMD::Get { key } => {
                    let response = match self.engine.get(key) {
                        Ok(v) => match v {
                            Some(v) => GetResponse::Ok(v),
                            None => GetResponse::Err(String::from("Key not found")),
                        },
                        Err(e) => GetResponse::Err(e.to_string()),
                    };
                    serde_json::to_writer(&mut writer, &response)?;
                    writer.flush()?;
                }
                CMD::Rm { key } => {
                    let response = match self.engine.remove(key) {
                        Ok(_) => SetResponse::Ok(()),
                        Err(e) => SetResponse::Err(e.to_string()),
                    };
                    serde_json::to_writer(&mut writer, &response)?;
                    writer.flush()?;
                }
            };
        }
        Ok(())
    }
}
