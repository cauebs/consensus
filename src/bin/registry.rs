use std::env::args;

use anyhow::Result;
use consensus::registry;

fn main() -> Result<()> {
    let mut args = args().skip(1);
    let usage = "Usage: registry <host>:<port> <peers-file>";

    let bind_addr = args
        .next()
        .expect(&format!("Expected bind address.\n{usage}"));

    let peers_file = args
        .next()
        .expect(&format!("Expected peers file.\n{usage}"));

    let mut server = registry::Server::new(peers_file)?;
    server.run(bind_addr)
}
