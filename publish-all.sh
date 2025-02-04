#!/bin/sh

cd packages/layer-climb-config && cargo publish
cd packages/layer-climb-proto && cargo publish
cd packages/layer-climb-address && cargo publish
cd packages/layer-climb-core && cargo publish
cd packages/layer-climb && cargo publish
cd packages/layer-climb-cli && cargo publish
