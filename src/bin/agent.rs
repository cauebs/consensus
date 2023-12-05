use std::env::args;

use anyhow::Result;
use consensus::{hierarchical_consensus::ConsensusAgent, parse_addr};
use rand::seq::SliceRandom;

fn main() -> Result<()> {
    env_logger::init();

    let mut args = args().skip(1);
    let usage = "Usage: agent <label> <bind-host>:<bind-port> <registry-host>:<registry-port>";

    let label = args
        .next()
        .unwrap_or_else(|| panic!("Expected label.\n{usage}"));

    let bind_addr = args
        .next()
        .and_then(parse_addr)
        .unwrap_or_else(|| panic!("Expected bind address.\n{usage}"));

    let registry_addr = args
        .next()
        .and_then(parse_addr)
        .unwrap_or_else(|| panic!("Expected registry address.\n{usage}"));

    let agent = ConsensusAgent::register(&label, bind_addr, registry_addr, |_| Ok(()))?;

    let id = agent.id;

    agent
        .with_proposal_factory(move || {
            let mut rng = rand::thread_rng();
            if id == 0 {
                None
            } else {
                let choices = [
                    "2001: Uma Odisseia no Espaço",
                    "Bacurau",
                    "O Encouraçado Potemkin",
                    "O Irlandês",
                    "Pantera Negra",
                    "Star Wars",
                    "Toy Story",
                    "Uma Linda Mulher",
                ]
                .into_iter()
                .map(|s| s.to_owned())
                .collect::<Vec<_>>();
                Some(choices.choose(&mut rng).cloned().unwrap())
            }
        })
        .run()
}
