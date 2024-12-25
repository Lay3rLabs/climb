# Climb Web Demo

## Prerequisites

* Trunk: https://trunkrs.dev/

* For local dev, start local chains (in examples dir): `starship start --config ./starship.yaml` (to stop it: `starship stop --config ./starship.yaml`)

## Run in browser

```
trunk serve --port 8000 --watch . --watch ../../packages
```

For quicker development, you can autoconnect a wallet with a mnemonic from `.env` (set in `LOCAL_MNEMONIC`)

```
trunk serve --features=autoconnect --port 8000 --watch . --watch ../../packages
```