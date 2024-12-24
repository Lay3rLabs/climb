# Climb Web Demo

## Prerequisites

* Trunk: https://trunkrs.dev/

## Run in browser

```
trunk serve --port 8000
```

For quicker development, you can autoconnect a wallet with a mnemonic from `.env` (set in `LOCAL_MNEMONIC`)

```
trunk serve --features=autoconnect --port 8000
```

And if you're making changes to the climb package, add it to the watcher too

```
trunk serve --features=autoconnect --port 8000 --watch . --watch ../../packages
```