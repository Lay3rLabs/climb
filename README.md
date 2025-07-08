# CLIent for Multiple Blockchains

* ### [Live demo](https://lay3rlabs.github.io/climb)
* ### [Cargo docs](https://docs.rs/layer-climb/latest/layer_climb/) 

## Universal, pure-Rust client lib for [Layer](https://layer.xyz) and beyond

Although hosted publicly, it's intended solely for Layer and Confio projects at the moment. PRs outside the official roadmap will be rejected.

## Wasm compatibility

All features besides [pools](#pools) work in browsers over gRPC-web (no need for gateway!) - just enable the `web` feature.

## Wasi compatibility

Should be similar to browser support, but less tested. Only RPC is supported at the moment, no gRPC.

## Examples

The examples are in the [examples](examples) directory. See [examples/README.md](examples/README.md) for more info. 

## RPC vs. gRPC

When creating either a query or signing client, you may choose to use force RPC or gRPC. Setting `None` will auto-detect availability, and prefer gRPC. 

## Prelude

Most of the types are re-exported in the prelude and can be used via the line-liner:

```rust
use layer_climb::prelude::*;
```

## SigningClient

[source code](packages/layer-climb-core/src/signing)

A SigningClient needs only two things, a ChainConfig and a TxSigner:

```rust
use layer_climb::prelude::*;

SigningClient::new(chain_config, signer, None).await
```

The `SigningClient` is cheap to clone and also fairly cheap to create.

#### ChainConfig

[source code](packages/layer-climb-config/src)

This is a serde-friendly data struct and is typically loaded from disk. See the [example in climb-cli](examples/config.json)

#### TxSigner

[source code](packages/layer-climb-core/src/transaction.rs)

This is a trait with only two required functions:

```rust
fn sign(&self, doc: &SignDoc) -> Result<Vec<u8>>;
fn public_key(&self) -> PublicKey;
```


For convenience, it can be created from a mnemonic with the provided [KeySigner](packages/layer-climb-address/src/key.rs) like:

```rust
use layer_climb::prelude::*;

// None here means "Cosmos derivation path"
let key_signer = KeySigner::new_mnemonic_str(my_mnemonic_string, None)?;
```

This plays nicely with the `bip39` and `rand` Rust crates, so you can easily generate a random mnemonic and pass it to `new_mnemonic_iter`:

```rust
use layer_climb::prelude::*;
use bip39::Mnemonic;
use rand::Rng;

let mut rng = rand::thread_rng();
let entropy: [u8; 32] = rng.gen();
let mnemonic = Mnemonic::from_entropy(&entropy)?;

let signer = KeySigner::new_mnemonic_iter(mnemonic.word_iter(), None)?;
```

In fact, that's exactly how the `generate-wallet` command in [climb-cli](packages/layer-climb-cli/src/command/wallet.rs) works!

## QueryClient

[source code](packages/layer-climb-core/src/querier.rs) 

If you have a SigningClient, then a QueryClient is created for you automatically as `signing_client.querier` and you have the wallet address in `signing_client.addr`

However, often you want to make queries against other addresses for which you don't have the Signing Key

All you need for this is the `ChainConfig`:

```rust
QueryClient::new(chain_config, None).await
```

The `QueryClient` is cheap to clone and also cheap to create (it uses a cache and reference-counting where applicable).

The QueryClient struct is slightly different for web targets, but this is all dealt with as an abstraction, methods are the same everywhere.

## Addresses

[source code](packages/layer-climb-address/src/address.rs)

One difference compared to other clients is that we require knowing the address type. This paves the way for supporting EVM address strings throughout the client. You can construct an address manually via methods like `new_cosmos()`, but it's more convenient to create it via a method on `ChainConfig`:

```rust
let addr = chain_config.parse_address("address string")?;
```

A similar method exists to derive it from a public key:

```rust
let addr = chain_config.address_from_pub_key(signer.public_key())?;
```

The `Display` implementation for `Address` is a plain string as would typically be expected for display purposes (events, block explorers, etc.)

## Transactions

Generally speaking, you just call a method on the `SigningClient`. For example, here's how to transfer funds:

```rust
use layer_climb::prelude::*;

let amount:u128 = 1_000_000;
let recpient_addr:Address = chain_config.parse_address("address string")?; // see `Addresses` above

// use chain's native gas denom
signing_client.transfer(amount, recipient_addr, None, None).await?;
// some other denom
signing_client.transfer(amount, recipient_addr, "uusdc", None).await?;
```

The last `None` is typical for all transaction methods. It takes a `TxBuilder` which allows configuring per-transaction settings like the gas fee, simulation multiplier, and many more.

[source code](packages/layer-climb-core/src/transaction.rs)

Technically, you don't even need a `SigningClient` for transactions, a `TxSigner` + `TxBuilder` + `QueryClient` is enough, but this is unwieldy. When you want to change transaction defaults, it's more convenient to get a `TxBuilder` from the `SigningClient`, and pass that as a parameter to the method (it will automatically pass the `TxSigner` along):


```rust
let tx_builder = signing_client.tx_builder();
tx_builder.set_gas_simulate_multiplier(2.0);
let tx_resp = signing_client.transfer(amount, recipient_addr, None, Some(tx_builder)).await?;
```

`tx_resp` contains the chain's native `TxResponse`, directly as the protobuf definition. You can log out the hash via `tx_resp.txhash`, or get really fancy by passing it to [CosmosTxEvents](#events).

#### See [Contracts](#contracts) for contract-specific transactions

## Requests / Responses

Internally, Query methods turn the arguments into a struct which implements a QueryRequest trait.

[source code](packages/layer-climb-core/src/querier.rs)

The exact implementation here is likely to change, but it's a way to support generic middleware over all requests so we can do things like retry requests on failure, write out logs, etc. More details on this below.

As an example, calling the [contract_code_info](packages/layer-climb-core/src/querier/contract.rs) method on `QueryClient` creates an internal [ContractCodeInfoReq](packages/layer-climb-core/src/querier/contract.rs) struct and the actual query is implemented on that struct's [request](packages/layer-climb-core/src/querier/contract.rs) method.

This is the pattern for all queries.

## Transaction messages

Transactions work in a similar way, however instead of creating an internal Request type, each method calls an internal helper to create some message, and then broadcasts the message with a TxBuilder.

This allows for calling those message-creating methods separately, and brodcasting them together in one transaction.

The [TxBuilder broadcast method](packages/layer-climb-core/src/transaction.rs) takes an iterator of these messages, which must be converted into a protobuf `Any`.

## Events

As a convenience helper to filter and search events, consider using `CosmosTxEvents`. It has `From` impls for various event sources like `TxResponse`, `Vec<Event>`, etc.

[source code](packages/layer-climb-core/src/events.rs)

It's a nearly zero-cost abstraction (just dynamic dispatch). Internally, it has variants with references, and so if you pass a reference source there are no allocations.

This is especially helpful for CosmWasm events, so you don't need to worry about the `wasm-` prefix. 

Here's an example of extracting the code id from a contract upload tx:

```rust
let code_id: u64 = CosmosTxEvents::from(&tx_resp)
    .attr_first("store_code", "code_id")?
    .value()
    .parse()?;
```

## Pools

For non-wasm targets (e.g. cli tools, desktop applications, bots, etc.) - you can use pools to get robust conccurency. Under the hood it uses different derivation paths for each client instead of account sequence numbers, thereby avoiding all the issues that can come up with trying to parallelize over the same client.

The pool itself is managed by a battle-tested third-party crate, [deadpool](https://crates.io/crates/deadpool) and is just plain Rust.

Example:
```rust
use layer_climb::prelude::*;
// import deadpool Pool 
use deadpool::managed::Pool;

// create a "pool manager", giving it a mnemonic, a chain config
// and an optional derivation index to start from (typically leave this as `None`)
let mut client_pool_manager = SigningClientPoolManager::new_mnemonic(mnemonic, chain_config.clone(), None);

// this part is completely optional, but highly recommended for real-world use
// it's a one-liner to set a minimum balance 
// and the pool will make sure each client has the funds before handing it out

// Minimum Balance Option 2
// make sure your "main address" (derivation index 0) has enough funds
// and just set the minimum balance
// * the 200_000 is the threshhold to tigger a send
// * the 1_000_000 is the amount that it will send when the balance falls below the threshhold 
client_pool_manager = client_pool_manager.with_minimum_balance(200_000, 1_000_000, None, None).await?;

// Minimum Balance Option 1
// give it a separate funder client, like a faucet, to send from
let faucet_signer = KeySigner::new_mnemonic_str(&faucet.mnemonic, None)?;
let faucet = SigningClient::new(chain_config, faucet_signer, None).await?;
client_pool_manager = client_pool_manager.with_minimum_balance(200_000, 1_000_000, Some(faucet), None).await?;

// In both of those, the last Option is just the denom
// similar to regular transfers, it will use the chain's gas denom if `None`

// anyway, with or without the "minimum balance" set, we can now create our pool
// this is just plain `deadpool`, with 100 max clients

let client_pool: Pool<SigningClientPoolManager> = Pool::builder(client_pool_manager)
.max_size(100)
.build()
.context("Failed to create client pool")?;
```

With the pool created, we can use it with plain Rust async concurrency primitives

Example:

```rust
/// upload 3 different contract files simultaneously
let (code_id_1, code_id_2, code_id_3) = try_join!(
    {
        let client_pool = client_pool.clone();
        async move {
            let client = client_pool.get().await?;
            let (code_id, tx_resp) =
                client.contract_upload_file(wasm_bytes_1, None).await?;
            Ok(code_id)
        }
    },
    {
        let client_pool = client_pool.clone();
        async move {
            let client = client_pool.get().await?;
            let (code_id, tx_resp) =
                client.contract_upload_file(wasm_bytes_2, None).await?;
            Ok(code_id)
        }
    },
    {
        let client_pool = client_pool.clone();
        async move {
            let client = client_pool.get().await?;
            let (code_id, tx_resp) =
                client.contract_upload_file(wasm_bytes_3, None).await?;
            Ok(code_id)
        }
    }
)?;
```

## Middleware

_the exact architecture here is likely to change_

The QueryClient supports middleware for:

* mapping requests
* mapping responses
* running request -> response

By default it runs a "runner" middleware to retry failing requests up to 3 times with a 100ms delay and exponential backoff.

The TxBuilder supports middleware for:

* mapping TxBody (containing all the messages)
* mapping TxResponse

By default, neither of these are set to anything.

_while the middleware implementations are internally trait-based, attempting to make the middleware field on QueryClient and TxBuilder trait-based, so that third-party middleware is supported, led to some problems with the futures becoming !Send_

## Logging

_this is very likely to change_

As of right now, some methods like IBC handlers take a function to log strings, and there are also logging middleware implementations.

The reason for this was to differentiate between developer logs and user-facing logs and due to the origins of this crate as it was embedded in an application, as opposed to the library it is now.

This will likely move to `tracing` and also decorate the library with tracing instrumention everywhere.

## Errors

_this is very likely to change_

As of right now, errors are merely emitted as `anyhow` strings. While convenient for quick development, it makes error recovery nearly impossible. Most likely this will move to `thiserror`.

## IBC

There are convenient methods for client, connection, and channel handshakes

[source code](packages/layer-climb-core/src/signing/ibc/handshake.rs)

With the handshake completed, we can create a fully-functioning IBC relayer:

[source code](packages/layer-climb-core/src/signing/ibc/relayer.rs)

The `IbcRelayer` type is constructed from a `IbcRelayerBuilder`. This allows for having a cache that can be re-used across instances, while also mutating the cache when starting up so that expired clients can be recreated.

[source code](packages/layer-climb-core/src/signing/ibc/relayer/builder.rs)

The ergonomics of this relayer are intended to support the use-case of relaying over preconfigured ports, not necessarily assuming that the ibc clients are maintained by the ecosystem such as ICS-20 on a popular mainnet.

_note: the relayer has been tested to complete packets from one chain to another, but there is currently an unresolved bug with relaying contract responses. It's recommended to use this only for simple testing, maintained relayers that are used at scale like Hermes or IBC-Relayer should be used in production_ 

## Contracts

Interacting with contracts is straightforward. Transactions (like instantiate, execute, etc.) are on SigningClient, and queries (like "smart queries" and "contract info") are on QueryClient

* [transactions source code](packages/layer-climb-core/src/signing/contract/tx.rs)
* [queries source code](packages/layer-climb-core/src/querier/contract.rs)

Let's look at some examples. For the sake of brevity, let's assume we already have a `SigningClient` called `client`, and let's assume our contract has the following types:

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

### Contract Upload
[source code](packages/layer-climb-core/src/signing/contract/tx.rs)

```rust
use layer_climb::prelude::*;

let wasm_byte_code = tokio::fs::read(wasm_file).await?;
let (code_id, tx_resp) = client.contract_upload_file(wasm_byte_code, None).await?;
```

The `code_id` in that response is a `u64`, and the `tx_resp` is the protobuf `TxResponse` mentioned above in [Transactions](#transactions)

### Contract Instantiation 
[source code](packages/layer-climb-core/src/signing/contract/tx.rs)

Now that we have a `code_id`, let's instantiate a contract:

```rust
use layer_climb::prelude::*;

let (addr, tx_resp) = client
    .contract_instantiate(
        client.addr.clone(), // admin
        code_id, 
        "my contract", // label 
        &InstantiateMsg {},
        Vec::new(), // optional funds
        None
    )
    .await?; 
```

### Contract Execution 

[source code](packages/layer-climb-core/src/signing/contract/tx.rs)

Now that we have a contract `addr`, let's execute a message:

```rust

use layer_climb::prelude::*;

let tx_resp = client.contract_execute(
    addr,
    &ExecuteMsg::StashMessage {
        message: "hello world".to_string()
    },
    Vec::new(), // optional funds
    None
).await?;
```

Sending funds is made easier with the [new_coin()](packages/layer-climb-core/src/prelude.rs) helper, and if you have multiple coins, use [new_coins()](packages/layer-climb-core/src/prelude.rs):

```rust
use layer_climb::prelude::*;

let tx_resp = client.contract_execute(
    addr,
    &ExecuteMsg::StashMessage {
        message: "hello world".to_string()
    },
    vec![new_coin("uslay", 1_000_000)],
    None
).await?;
```

### Contract Query

[source code](packages/layer-climb-core/src/querier/contract.rs)

Now that we've executed something on the contract, let's query it. Notice that this is a method on the `QueryClient`, meaning we don't actually need a `SigningClient` - but we'll just use the one we have: 

```rust
use layer_climb::prelude::*;

let query_resp:MessagesResp = client.querier.contract_smart(
    &addr, 
    &QueryMsg::GetMessages {
        after_index: None, 
        order: None 
    },
).await?;
```

The response is typechecked at runtime via `serde_json` (well, actually the cosmwasm_std implementation, to make sure it's 100% compatible with smart contracts), and from then on we get perfect guarantees that the response is what we expect.

What if we wanted to get it as a raw string instead? Just call the [.contract_smart_raw_response()](packages/layer-climb-core/src/querier/contract.rs) method:


```rust
use layer_climb::prelude::*;

let raw_bytes = client.querier.contract_smart_raw(
    &addr, 
    &QueryMsg::GetMessages {
        after_index: None, 
        order: None 
    },
).await?;

let raw_string = std::str::from_utf8(&raw_bytes)?;
```

## Generic Messages

[source code](packages/layer-climb-core/src/contract_helpers.rs#14)

Sometimes, especially with tooling that isn't project-specific, we want to send contract messages without knowing the type at all.

Under the hood, this is done by serializing as a JSON-formatted string, and for CosmWasm contracts, we send `"{}"` instead of null for "empty messages". This is all made easier with the `contract_str_to_msg` helper. Some examples:


```rust
use layer_climb::prelude::*;

// Example execution
let tx_resp = client.contract_execute(
    addr,
    &contract_str_to_msg("{\"stash_message\": {\"message\": \"hello world\"}}")?,
    None, // optional funds
    None
).await?;

// Example query
let raw_bytes = client.querier.contract_smart_raw(
    &addr, 
    &contract_str_to_msg("{\"get_messages\": {}}")?
).await?;
let raw_string = std::str::from_utf8(&raw_bytes)?;

// bonus: given this input from a CLI tool, we can handle it all with .as_deref()
let maybe_message:Option<String>;

let tx_resp = client.contract_execute(
    addr,
    &contract_str_to_msg(maybe_message.as_deref())?,
    None, // optional funds
    None
).await?;
```
