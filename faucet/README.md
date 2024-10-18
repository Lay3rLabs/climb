# Climb Faucet

This is a faucet service similar to [the CosmJS Faucet](https://github.com/cosmos/cosmjs/tree/main/packages/faucet) with a few key differences:

* Written in Rust, using Axum for the server and Climb for the blockchain client
* Uses Climb signing client pools to manage concurrency and move funds around derived wallets
* Doesn't maintain 1:1 feature parity, but rather aims to support the core API requirements

### Configuration

See [faucet.toml](./faucet.toml) for various configuration settings.

You can load a different configuration file by passing `--config [path]`