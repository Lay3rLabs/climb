# Climb CLI

This is an example CLI

It merely provides a thin layer around the reusable [layer-climb-cli](../../packages/layer-climb-cli) crate.

Real-world usage such as [avs-toolkit](https://github.com/Lay3rLabs/avs-toolkit/tree/main/tools/cli) takes this as a starting point and builds from there.

## Running a local node

For all the below commands, you need to make sure you have a localnode running.
Instructions are in the [localnode directory](https://github.com/Lay3rLabs/layer-sdk/tree/main/localnode),
but the shortcut is:

```bash
# From workspace root
./scripts/build_docker.sh
./localnode/reset_volumes.sh
./localnode/run.sh

# do whatever below. when you want to stop it

./localnode/stop.sh
```

If you hit any errors, please check the full README

## Setup

To start, you need to get a unique mnemonic and store it. The easiest way is:

```bash
cargo run generate-wallet
```

In this directory, create a file called `.env` with `LOCAL_MNEMONIC="<mnemonic provided above>".
Now, check you can view it

```bash
cargo run wallet-show
```

## Getting tokens

The next step is to "tap the faucet" to get some tokens for your new address,
so you can use the CLI more:

```bash
cargo run tap-faucet
```

Yeah, you got some tokens now. Let's go do some more stuff...

## Contracts

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

## Instantiate a contract

The `InstantiateMsg` in our contract is empty. So, all we need is to run:

```bash
cargo run instantiate-contract --code-id=2
```

Note the "Contract Address" in the output. Let's say it's "slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue"

## Execute a contract

Here we have a non-empty ExecuteMsg, so we need to supply it as a JSON-encoded string. For example:

```bash
cargo run execute-contract --address="slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue5" --msg="{\"stash_message\": {\"message\": \"hello world\"}}"
```

We should get a successful "Tx Hash" in the response

## Query a contract

Here we have a non-empty QueryMsg, so we need to supply it as a JSON-encoded string. Optionals can be left out entirely. For example:

```bash
cargo run query-contract --address="slay3r1lu0l5xgnjwugk70uujyyqyw9uvapwh6m05es5xkhk0zk4n60a87qsrcue5" --msg="{\"get_messages\": {}}"
```

We'll get output like: `Query Response: "{\"messages\":[\"hello world\"]}"`
