use std::{env::args, time::Duration};

use consensus::{parse_addr, perfect_failure_detector::PerfectFailureDetector};

fn main() {
    env_logger::init();

    let mut args = args().skip(1);
    let usage =
        "Usage: pfd <bind-host>:<bind-port> <registry-host>:<registry-port> <timeout-seconds>";

    let bind_addr = args
        .next()
        .and_then(parse_addr)
        .unwrap_or_else(|| panic!("Expected bind address.\n{usage}"));

    let registry_addr = args
        .next()
        .and_then(parse_addr)
        .unwrap_or_else(|| panic!("Expected registry address.\n{usage}"));

    let timeout = args
        .next()
        .and_then(|s| s.parse().ok())
        .map(Duration::from_secs)
        .unwrap_or_else(|| panic!("Expected heartbeat timeout in seconds.\n{usage}"));

    PerfectFailureDetector::new(bind_addr, registry_addr, timeout).run();
}
