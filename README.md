# What is it

This is a simulation of two different algorithms for delivering messages in sparse network, where agents can only broadcast messages to their neighbours.

One algorithm is a slightly modified Gossip, and the other is Destination-Sequenced Distance Vector routing.

# Run

[Install rust](https://www.rust-lang.org/tools/install)

Check build instructions for macroquad in [it's readme](https://github.com/not-fl3/macroquad#build-instructions) (you may need to install some packages)

## Linux (WSL) cross compile to windows

Run 12 and 19 (apt-get) commands from [this file](https://github.com/cross-rs/cross/blob/master/docker/Dockerfile.x86_64-pc-windows-gnu#L12) 

Also run this

```bash
rustup target add x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu
```

Then, add `--target x86_64-pc-windows-gnu` to every cargo command, like `cargo build --bin console --target x86_64-pc-windows-gnu`

## Commands

```bash
# Add RUST_LOG=(debug|info|error) for log output
# Run tests with full output
RUST_LOG=debug cargo test -- --nocapture
# Just run (even in wsl)
cargo run --bin console
cargo run --release --bin visual  # --release for release build
# Or, you can just build
cargo build --bin console
cargo build --release --bin visual
# The output will be somewhere here
./target/debug/console
./target/release/visual
# Or, for cross compilation
./target/x86_64-pc-windows-gnu/debug/console
./target/x86_64-pc-windows-gnu/release/console
```
