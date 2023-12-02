use std::{
    collections::HashSet,
    net::{SocketAddr, TcpListener},
    sync::RwLock,
    thread::{self, sleep},
    time::Duration,
};

use crate::{broadcast, deserialize_from, registry, Message, PeerId};
use anyhow::Result;
use serde::{Deserialize, Serialize};

pub struct PerfectFailureDetector {
    bind_addr: SocketAddr,
    registry: registry::Client,
    confirmed_alive: RwLock<HashSet<PeerId>>,
    timeout: Duration,
}

#[derive(Serialize, Deserialize)]
pub struct Heartbeat {
    pub peer_id: PeerId,
}

impl PerfectFailureDetector {
    pub fn new(bind_addr: SocketAddr, registry_addr: SocketAddr, timeout: Duration) -> Self {
        Self {
            bind_addr,
            registry: registry::Client::new(registry_addr),
            confirmed_alive: Default::default(),
            timeout,
        }
    }

    pub fn run(&self) {
        thread::scope(|s| {
            s.spawn(|| self.listen_to_heartbeats().unwrap());
            s.spawn(|| self.run_timeout_loop().unwrap());
        })
    }

    fn listen_to_heartbeats(&self) -> Result<()> {
        for connection in TcpListener::bind(self.bind_addr)?.incoming() {
            let heartbeat: Heartbeat = deserialize_from(connection?)?;
            self.set_alive(heartbeat.peer_id, true);
        }
        unreachable!()
    }

    fn run_timeout_loop(&self) -> Result<()> {
        let mut assumed_dead = HashSet::new();
        let mut known_peers = HashSet::new();
        loop {
            let peers = self.registry.get_peers()?;

            for peer in &peers {
                if assumed_dead.contains(&peer.id) {
                    continue;
                }

                let confirmed_alive = self.set_alive(peer.id, false);
                if confirmed_alive || !known_peers.contains(&peer.id) {
                    known_peers.insert(peer.id);
                    if peer
                        .send(Message::RequestHeartbeat::<()> {
                            requester: self.bind_addr,
                        })
                        .is_ok()
                    {
                        continue;
                    }
                }

                assumed_dead.insert(peer.id);
                broadcast(Message::InformCrash::<()>(peer.id), peers.iter().cloned());
            }

            sleep(self.timeout);
        }
    }

    /// Sets a liveness state and returns the previous value.
    fn set_alive(&self, peer_id: PeerId, state: bool) -> bool {
        let mut alive = self.confirmed_alive.write().unwrap();
        match state {
            true => alive.insert(peer_id),
            false => alive.remove(&peer_id),
        }
    }
}
