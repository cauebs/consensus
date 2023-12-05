# Hierarchical Consensus

To see the log:
```shell
export RUST_LOG=info
```
or
```shell
export RUST_LOG=debug
```
or
```shell
export RUST_LOG=trace
```

## Running the demo
```shell
$ ./deploy.py <num-agents> <heartbeat-timeout-seconds>
```
e.g.
```shell
$ ./deploy.py 4 10
```

- Processes will bind to ports starting at 8000

## Running modular
Run in the following order:

```shell
cargo run --release --bin registry <host>:<port> <peers-file>
```

```shell
cargo run --release --bin pfd <bind-host>:<bind-port> <registry-host>:<registry-port> <timeout-seconds>
```

```shell
cargo run --release --bin agent <bind-host>:<bind-port> <registry-host>:<registry-port>
```
