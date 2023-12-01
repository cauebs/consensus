use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::{deserialize_from, serialize_into, Peer};

#[derive(Serialize, Deserialize)]
pub enum Request {
    Register(SocketAddr),
    GetPeers,
}

#[derive(Serialize, Deserialize)]
pub enum Response {
    Registered,
    Peers(Vec<Peer>),
}

#[derive(Default)]
pub struct Server {
    peers: Vec<Peer>,
}

impl Server {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn run(&mut self, addr: impl ToSocketAddrs) -> Result<()> {
        for client in TcpListener::bind(addr)?.incoming() {
            let Ok(client) = client else { continue };
            let request: Request = deserialize_from(&client)?;

            let response = match request {
                Request::Register(addr) => {
                    self.register(addr);
                    Response::Registered
                }
                Request::GetPeers => Response::Peers(self.peers.clone()),
            };

            serialize_into(&client, &response)?;
        }
        unreachable!()
    }

    fn register(&mut self, peer_addr: SocketAddr) {
        let id = self
            .peers
            .last()
            .map(|peer| peer.id + 1)
            .unwrap_or_default();
        self.peers.push(Peer::new(id, peer_addr));
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
