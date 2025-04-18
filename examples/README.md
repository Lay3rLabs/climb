# Climb Web Demo

## Prerequisites

* Trunk: https://trunkrs.dev/

* For local dev, start local chains (in examples dir): `starship start --config ./starship.yaml` (to stop it: `starship stop --config ./starship.yaml`)

## Run in browser

All these commands assume you're in the `examples/frontend` directory.

```
trunk serve --port 8000 --watch . --watch ../../packages
```

For quicker development, you can autoconnect a wallet with a mnemonic from `.env` (set in `LOCAL_MNEMONIC`)

```
trunk serve --features=autoconnect --port 8000 --watch . --watch ../../packages
```

## WASI

All these commands assume you're in the `examples/wasi` directory.

### Build

There isn't a local example of running, since you need wasmtime etc... but if it builds it should work :)

```
cargo component build 
```

Note that WASI only supports rpc, not grpc

## CLI

All these commands assume you're in the `examples/cli` directory.

### Setup

To start, you need to get a unique mnemonic and store it. The easiest way is to copy the `example.env` into the `cli` directory.

You can also create a new wallet:

```bash
cargo run wallet create
```

In this directory, create a file called `.env` with `LOCAL_MNEMONIC="<mnemonic provided above>".
Now, check you can view it

```bash
cargo run wallet show
```

### Getting tokens

The next step is to "tap the faucet" to get some tokens for your new address,
so you can use the CLI more:

```bash
cargo run faucet tap
```

Yeah, you got some tokens now. Let's go do some more stuff...

### Contracts

Let's assume we have a contract built with the following message types:

```rust
#[cw_serde]
pub struct InstantiateMsg { }

#[cw_serde]
pub enum ExecuteMsg {
    StashMessage {
        message: String
    }
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// * returns [MessagesResp]
    #[returns(MessagesResp)]
    GetMessages {
        after_index: Option<Uint64>,
        order: Option<Order>
    },
}

#[cw_serde]
pub struct MessagesResp {
    pub messages: Vec<String>,
}
```

### Instantiate

```bash
cargo run upload-contract --wasm-file "path/to/your/contract.wasm"
```

Note the `Code ID` in the output. Let's say it's "2"

#### Instantiate a contract

The `InstantiateMsg` in our contract is empty. So, all we need is to run:

```bash
cargo run instantiate-contract --code-id=2
```

Note the "Contract Address" in the output. Let's say it's "slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue"

#### Execute a contract

Here we have a non-empty ExecuteMsg, so we need to supply it as a JSON-encoded string. For example:

```bash
cargo run execute-contract --address="slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue5" --msg="{\"stash_message\": {\"message\": \"hello world\"}}"
```

We should get a successful "Tx Hash" in the response

#### Query a contract

Here we have a non-empty QueryMsg, so we need to supply it as a JSON-encoded string. Optionals can be left out entirely. For example:

```bash
cargo run query-contract --address="slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue5" --msg="{\"get_messages\": {}}"
```

We'll get output like: `Query Response: "{\"messages\":[\"hello world\"]}"`