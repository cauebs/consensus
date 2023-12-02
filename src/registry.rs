use std::{
    fs,
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{deserialize_from, serialize_into, Peer};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    Register(SocketAddr),
    GetPeers,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Registered,
    Peers(Vec<Peer>),
}

pub struct Server {
    peers_file: PathBuf,
}

impl Server {
    pub fn new(peers_file: impl AsRef<Path>) -> Result<Self> {
        let server = Self {
            peers_file: peers_file.as_ref().to_owned(),
        };

        if !server.peers_file.exists() {
            server.write_peers(&[])?;
        }
        Ok(server)
    }

    fn read_peers(&self) -> Result<Vec<Peer>> {
        let file_contents = fs::read(&self.peers_file)?;
        Ok(serde_json::from_slice(&file_contents)?)
    }

    fn write_peers(&self, peers: &[Peer]) -> Result<()> {
        let file_contents = serde_json::to_vec_pretty(&peers)?;
        Ok(fs::write(&self.peers_file, &file_contents)?)
    }

    pub fn run(&mut self, addr: impl ToSocketAddrs) -> Result<()> {
        for client in TcpListener::bind(addr)?.incoming() {
            let Ok(client) = client else { continue };
            let request: Request = deserialize_from(&client)?;

            let response = match request {
                Request::Register(addr) => {
                    self.register(addr)?;
                    Response::Registered
                }
                Request::GetPeers => Response::Peers(self.read_peers()?),
            };

            serialize_into(&client, &response)?;
        }
        unreachable!()
    }

    fn register(&mut self, peer_addr: SocketAddr) -> Result<()> {
        let mut peers = self.read_peers()?;

        let id = peers.last().map(|peer| peer.id + 1).unwrap_or_default();
        peers.push(Peer::new(id, peer_addr));

        self.write_peers(&peers)
    }
}

pub struct Client {
    server_addr: SocketAddr,
}

impl Client {
    pub fn new(server_addr: SocketAddr) -> Self {
        Self { server_addr }
    }

    pub fn register(&self, peer_addr: SocketAddr) -> Result<()> {
        let server = TcpStream::connect(self.server_addr)?;
        serialize_into(&server, Request::Register(peer_addr))?;
        let response: Response = deserialize_from(&server)?;
        match response {
            Response::Registered => Ok(()),
            _ => panic!(),
        }
    }

    pub fn get_peers(&self) -> Result<Vec<Peer>> {
        let server = TcpStream::connect(self.server_addr)?;
        serialize_into(&server, Request::GetPeers)?;
        let response: Response = deserialize_from(&server)?;
        match response {
            Response::Peers(peers) => Ok(peers),
            _ => panic!(),
        }
    }
}
