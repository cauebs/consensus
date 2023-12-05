use crate::{
    broadcast, deserialize_from, perfect_failure_detector::Heartbeat, registry, serialize_into,
    Message, PeerId,
};
use anyhow::Result;
use log::{debug, error, info, trace};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::Debug,
    net::{SocketAddr, TcpListener, TcpStream},
    thread::sleep,
    time::Duration,
};

type Round = usize;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Proposal<T: Clone + Debug> {
    value: T,
    proposer: PeerId,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ConsensusEvent<T: Clone + Debug> {
    Start,
    Decided(Proposal<T>),
    Undecided,
}

pub struct ConsensusAgent<T, F, C>
where
    T: Clone + Debug,
    F: Fn() -> Option<T>,
    C: Fn(T) -> Result<()>,
{
    bind_addr: SocketAddr,
    pub id: PeerId,
    label: String,
    registry: registry::Client,
    crashed_peers: HashSet<PeerId>,
    current_round: Round,
    proposal_factory: Option<F>,
    proposal: Option<Proposal<T>>,
    has_decided: bool,
    decision_callback: C,
}

impl<T, F, C> ConsensusAgent<T, F, C>
where
    T: Serialize + DeserializeOwned + Clone + Debug + Send + Sync + 'static,
    F: Fn() -> Option<T>,
    C: Fn(T) -> Result<()>,
{
    pub fn register(
        label: &str,
        bind_addr: SocketAddr,
        registry_addr: SocketAddr,
        decision_callback: C,
    ) -> Result<Self> {
        let registry = registry::Client::new(registry_addr);

        Ok(Self {
            bind_addr,
            id: registry.register(bind_addr)?,
            label: label.to_owned(),
            registry,
            crashed_peers: Default::default(),
            current_round: 0,
            proposal_factory: None,
            proposal: None,
            has_decided: false,
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
            sleep(Duration::from_secs(1));
            self.try_decide()?;
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
        } else {
            self.proposal = None;
        }
        info!(
            "Agent {}({}) assuming initial proposal: {:?}",
            self.id,
            self.label,
            self.proposal.as_ref().map(|p| &p.value)
        )
    }

    fn reset(&mut self) {
        self.crashed_peers.clear();
        self.current_round = 0;
        self.refresh_proposal();
        self.has_decided = false;
    }

    fn try_decide(&mut self) -> Result<()> {
        if self.has_decided {
            return Ok(());
        }

        let message = if let Some(proposal) = &self.proposal {
            info!(
                "Agent {}({}) deciding: {:?}",
                self.id, self.label, proposal.value
            );
            self.has_decided = true;
            let callback = &self.decision_callback;
            callback(proposal.value.clone())?;

            if self.current_round > self.id {
                return Ok(());
            }

            info!("Agent {}({}) broadcasting decision", self.id, self.label);
            Message::ConsensusEvent(ConsensusEvent::Decided(proposal.clone()))
        } else {
            info!(
                "Agent {}({}) is undecided and will decided later",
                self.id, self.label
            );
            Message::ConsensusEvent(ConsensusEvent::Undecided)
        };

        let peers = self.registry.get_peers()?;
        std::thread::spawn(move || broadcast(&message, peers.into_iter()));
        Ok(())
    }

    fn advance_round(&mut self) -> Result<()> {
        info!(
            "Agent {}({}) advancing from round {} to round {}",
            self.id,
            self.label,
            self.current_round,
            self.current_round + 1
        );
        self.current_round += 1;

        if self.crashed_peers.contains(&self.current_round) {
            self.advance_round()?;
        }

        if self.current_round >= self.id {
            self.try_decide()?;
        }

        Ok(())
    }

    fn handle(&mut self, message: Message<T>) -> Result<()> {
        trace!("{}: {:?}", self.id, &message);
        match message {
            Message::RequestHeartbeat { requester } => {
                debug!("Agent {}({}) sending heartbeat", self.id, self.label);
                let stream = TcpStream::connect(requester)?;
                serialize_into(stream, Heartbeat { peer_id: self.id })?;
            }

            Message::InformCrash(peer_id) => {
                error!(
                    "Agent {}({}) informed that peer {} crashed",
                    self.id, self.label, peer_id
                );
                self.crashed_peers.insert(peer_id);
                if peer_id == self.current_round && !self.has_decided {
                    self.advance_round()?;
                }
            }

            Message::ConsensusEvent(ConsensusEvent::Start) => {
                self.reset();
            }

            Message::ConsensusEvent(ConsensusEvent::Decided(new_proposal)) => {
                if self.has_decided {
                    return Ok(());
                }

                if let Some(current_proposal) = &self.proposal {
                    if current_proposal.proposer < new_proposal.proposer {
                        debug!(
                            "Agent {}({}) ignoring proposal originally from peer {}",
                            self.id, self.label, new_proposal.proposer
                        );
                        return self.advance_round();
                    }
                }

                debug!(
                    "Agent {}({}) incorporating decision from peer {}",
                    self.id, self.label, new_proposal.proposer
                );
                self.proposal = Some(new_proposal);
                self.advance_round()?;
            }

            Message::ConsensusEvent(ConsensusEvent::Undecided) => {
                if self.current_round != self.id {
                    self.advance_round()?;
                }
            }
        }

        Ok(())
    }
}
