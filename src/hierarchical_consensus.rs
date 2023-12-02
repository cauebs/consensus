use crate::{
    broadcast, deserialize_from, perfect_failure_detector::Heartbeat, registry, serialize_into,
    Message, PeerId,
};
use anyhow::Result;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::{SocketAddr, TcpListener, TcpStream},
};

type Round = usize;

#[derive(Serialize, Deserialize, Clone)]
pub struct Proposal<T: Clone> {
    value: T,
    proposer: PeerId,
}

#[derive(Serialize, Deserialize)]
pub enum ConsensusEvent<T: Clone> {
    Start,
    Decided(Proposal<T>),
}

pub struct ConsensusAgent<T, F, C>
where
    T: Clone,
    F: Fn() -> Option<T>,
    C: Fn(T) -> Result<()>,
{
    bind_addr: SocketAddr,
    id: PeerId,
    registry: registry::Client,
    crashed_peers: HashSet<PeerId>,
    current_round: Round,
    proposal_factory: Option<F>,
    proposal: Option<Proposal<T>>,
    last_broadcast: Option<Round>,
    decision_callback: C,
}

impl<T, F, C> ConsensusAgent<T, F, C>
where
    T: Serialize + DeserializeOwned + Clone,
    F: Fn() -> Option<T>,
    C: Fn(T) -> Result<()>,
{
    pub fn register(
        bind_addr: SocketAddr,
        registry_addr: SocketAddr,
        decision_callback: C,
    ) -> Result<Self> {
        let registry = registry::Client::new(registry_addr);

        Ok(Self {
            bind_addr,
            id: registry.register(bind_addr)?,
            registry,
            crashed_peers: Default::default(),
            current_round: 0,
            proposal_factory: None,
            proposal: None,
            last_broadcast: None,
            decision_callback,
        })
    }

    pub fn with_proposal_factory(mut self, factory: F) -> Self {
        self.proposal_factory = Some(factory);
        self.refresh_proposal();
        self
    }

    pub fn run(&mut self) -> Result<()> {
        if self.id == self.current_round {
            self.decide()?;
        }

        for stream in TcpListener::bind(self.bind_addr)?.incoming() {
            let Ok(stream) = stream else { continue };
            self.handle(deserialize_from(stream)?)?;
        }
        unreachable!()
    }

    fn refresh_proposal(&mut self) {
        if let Some(factory) = &self.proposal_factory {
            self.proposal = factory().map(|value| Proposal {
                value,
                proposer: self.id,
            });
        }
    }

    fn reset(&mut self) {
        self.crashed_peers.clear();
        self.current_round = 0;
        self.refresh_proposal();
        self.last_broadcast = None;
    }

    fn decide(&mut self) -> Result<()> {
        let peers = self.registry.get_peers()?;
        let proposal = self.proposal.take().unwrap();
        let message = Message::ConsensusEvent(ConsensusEvent::Decided(proposal.clone()));

        broadcast(&message, peers.into_iter());
        self.last_broadcast = Some(self.current_round);

        let callback = &self.decision_callback;
        callback(proposal.value)?;
        Ok(())
    }

    fn advance_round(&mut self) -> Result<()> {
        self.current_round += 1;

        if self.crashed_peers.contains(&self.current_round) {
            self.advance_round()?;
        }

        if self.id == self.current_round {
            self.decide()?;
        }

        Ok(())
    }

    fn handle(&mut self, message: Message<T>) -> Result<()> {
        match message {
            Message::RequestHeartbeat { requester } => {
                let stream = TcpStream::connect(requester)?;
                serialize_into(stream, Heartbeat { peer_id: self.id })?;
            }

            Message::InformCrash(peer_id) => {
                self.crashed_peers.insert(peer_id);
                if peer_id == self.current_round {
                    self.advance_round()?;
                }
            }

            Message::ConsensusEvent(ConsensusEvent::Start) => {
                self.reset();
            }

            Message::ConsensusEvent(ConsensusEvent::Decided(new_proposal)) => {
                self.advance_round()?;

                if new_proposal.proposer > self.id {
                    return Ok(());
                }

                if let Some(current_proposal) = &self.proposal {
                    if new_proposal.proposer > current_proposal.proposer {
                        self.proposal = Some(new_proposal);
                    }
                }
            }
        }

        Ok(())
    }
}
