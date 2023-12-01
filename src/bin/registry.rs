use std::env::args;

use anyhow::Result;
use consensus::registry;

fn main() -> Result<()> {
    let bind_addr = args().nth(1).expect("Usage: registry <host>:<port>");
    let mut server = registry::Server::new();
    server.run(bind_addr)
}
