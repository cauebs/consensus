#!/usr/bin/python3

from itertools import count
from pathlib import Path
from shlex import split
from subprocess import Popen, run
from sys import argv
from time import sleep
from typing import Iterator


def bind_addresses(ip: str = "0.0.0.0", starting_port: int = 5000) -> Iterator[str]:
    for port in count(start=starting_port):
        yield f"{ip}:{port}"


def start_bin(name: str, args: list[str] = []) -> Popen:
    print(f"!!! starting {name} with {args}")
    command = split(f"cargo run --release --quiet --bin {name}") + args
    return Popen(command)


def restart(proc: Popen) -> Popen:
    return Popen(proc.args)


def deploy(num_agents: int, heartbeat_timeout: int) -> None:
    run(split("cargo build --release --bins"))

    address = bind_addresses()
    services = {}

    registry_addr = next(address)
    peers_file = Path("/tmp/consensus-registry")
    peers_file.unlink(missing_ok=True)
    services["registry"] = start_bin("registry", [registry_addr, str(peers_file)])
    sleep(1)

    for _ in range(num_agents):
        start_bin("agent", [next(address), registry_addr])

    services["pfd"] = start_bin(
        "pfd", [next(address), registry_addr, str(heartbeat_timeout)]
    )

    while True:
        for name in services:
            proc = services[name]
            exit_code = proc.poll()
            if exit_code is None:
                continue
            services[name] = restart(proc)

        sleep(heartbeat_timeout / 2)


if __name__ == "__main__":
    assert len(argv) == 3, "Usage: deploy.py <num-agents> <heartbeat-timeout-seconds>"
    num_agents = int(argv[1])
    heartbeat_timeout = int(argv[2])

    deploy(num_agents=num_agents, heartbeat_timeout=heartbeat_timeout)
