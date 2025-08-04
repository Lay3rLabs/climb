Due to interdependencies, publishing should be done in roughly this order:

* layer-climb-config
* layer-climb-proto
* layer-climb-address
* layer-climb-core
* layer-climb
* layer-climb-cli

Remember to bump the version of the dependencies too, not just workspace version (but they are all set in the root Cargo.toml) 

For convenience, publish all in one command:

```shell
cd packages/layer-climb-address && cargo publish \
    && cd ../layer-climb-config && cargo publish \
    && cd ../layer-climb-proto && cargo publish \
    && cd ../layer-climb-signer && cargo publish \
    && cd ../layer-climb-core && cargo publish \
    && cd ../layer-climb && cargo publish \
    && cd ../layer-climb-cli && cargo publish
```
