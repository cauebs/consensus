use std::env::args;

use anyhow::Result;
use consensus::{hierarchical_consensus::ConsensusAgent, parse_addr};
use rand::Rng;

fn main() -> Result<()> {
    let mut args = args().skip(1);
    let usage = "Usage: agent <bind-host>:<bind-port> <registry-host>:<registry-port>";

    let bind_addr = args
        .next()
        .and_then(parse_addr)
        .expect(&format!("Expected bind address.\n{usage}"));

    let registry_addr = args
        .next()
        .and_then(parse_addr)
        .expect(&format!("Expected registry address.\n{usage}"));

    ConsensusAgent::register(bind_addr, registry_addr, |decision: u32| {
        Ok(println!("decided: {decision:?}"))
    })?
    .with_proposal_factory(|| {
        let mut rng = rand::thread_rng();
        Some(rng.gen::<u32>())
    })
    .run()
}
