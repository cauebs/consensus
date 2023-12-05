use std::env::args;

use anyhow::Result;
use consensus::registry;

fn main() -> Result<()> {
    env_logger::init();

    let mut args = args().skip(1);
    let usage = "Usage: registry <host>:<port> <peers-file>";

    let bind_addr = args
        .next()
        .unwrap_or_else(|| panic!("Expected bind address.\n{usage}"));

    let peers_file = args
        .next()
        .unwrap_or_else(|| panic!("Expected peers file.\n{usage}"));

    let mut server = registry::Server::new(peers_file)?;
    server.run(bind_addr)
}
