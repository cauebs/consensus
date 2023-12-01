use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use thiserror::Error;

pub mod perfect_failure_detector;
pub mod registry;

pub fn parse_addr(addr: impl ToSocketAddrs) -> Option<SocketAddr> {
    addr.to_socket_addrs().ok().and_then(|mut addresses| addresses.next())
}

pub type PeerId = usize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Peer {
    id: PeerId,
    addr: SocketAddr,
}

#[derive(Serialize, Deserialize)]
pub enum Message {
    RequestHeartbeat,
    InformCrash(PeerId),
}

impl Peer {
    pub fn new(id: PeerId, addr: SocketAddr) -> Self {
        Self { id, addr }
    }

    pub fn send<M: Serialize>(&self, message: M) -> anyhow::Result<()> {
        let stream = TcpStream::connect(self.addr)?;
        serialize_into(stream, message)
    }
}

pub fn serialize_into<W: Write, T: Serialize>(writer: W, value: T) -> anyhow::Result<()> {
    Ok(bincode::serialize_into(writer, &value)?)
}

pub fn deserialize_from<R: Read, T: DeserializeOwned>(reader: R) -> anyhow::Result<T> {
    Ok(bincode::deserialize_from(reader)?)
}

#[derive(Debug, Error)]
#[error("Failed to send message to {failures:?}")]
pub struct BroadcastError {
    pub failures: Vec<SocketAddr>,
}

pub fn try_broadcast<M: Serialize>(
    message: M,
    peers: impl Iterator<Item = Peer>,
) -> Result<(), BroadcastError> {
    let send_or_addr = |peer: Peer| peer.send(&message).err().map(|_e| peer.addr);
    let failures: Vec<_> = peers.flat_map(send_or_addr).collect();
    if failures.is_empty() {
        Ok(())
    } else {
        // TODO: does this make sense? should we just ignore these?
        Err(BroadcastError { failures })
    }
}

pub fn broadcast<M: Serialize>(
    message: M,
    peers: impl Iterator<Item = Peer>,
) {
    for peer in peers {
        let _ = peer.send(&message);
    }
}
