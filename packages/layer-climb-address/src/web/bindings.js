let lastId = 0;
const lookup = new Map();

export async function ffi_keplr_register_signer(chainId) {
    if (!window.keplr) {
        throw new Error("Please install keplr extension");
    }
    await window.keplr.enable(chainId);
    const signer = await window.keplr.getOfflineSigner(chainId);
    const keplrKey = await window.keplr.getKey(chainId);

    lastId++;

    let id = lastId.toString();

    lookup.set(id, {chainId, keplrKey, signer});

    return id;
}

export async function ffi_keplr_sign(keplrId, signDoc) {
    const data = lookup.get(keplrId);
    if (!data) {
        throw new Error("Signer not found");
    }

    const {keplrKey, signer} = data;

    // https://github.com/chainapsis/keplr-wallet/blob/540fc84a2a30f5a221cfa7bc37707aab5b8f25d8/packages/provider-extension/src/cosmjs.ts#L72
    const res = await signer.signDirect(keplrKey.bech32Address, signDoc);
    return res.signature.signature
}

export async function ffi_keplr_public_key(keplrId) {
    const data = lookup.get(keplrId);
    if (!data) {
        throw new Error("Signer not found");
    }

    return data.keplrKey;
}

export async function ffi_keplr_add_chain(config) {
    if (!window.keplr) {
        throw new Error("Please install keplr extension");
    }

    const addrPrefix = config.address_kind?.cosmos?.prefix;

    if(!addrPrefix || addrPrefix.length === 0) {
        throw new Error("Chain config doesn't have valid cosmos address prefix");
    }

    const currency = {
        coinDenom: config.gas_denom,
        coinMinimalDenom: config.gas_denom,
        coinDecimals: 6,
        coinGeckoId: config.gas_denom,
    }

    const restUrl = new URL(config.rpc_endpoint);
    restUrl.port = '1317';
    const restEndpoint = restUrl.toString();

    const keplrConfig = {
        chainId:  config.chain_id,
        chainName: config.chain_id,
        rpc: config.rpc_endpoint,
        rest: restEndpoint, 
        bip44: {
            coinType: 118,
        },
        bech32Config: {
            bech32PrefixAccAddr: addrPrefix,
            bech32PrefixAccPub: `${addrPrefix}pub`,
            bech32PrefixValAddr: `${addrPrefix}valoper`,
            bech32PrefixValPub: `${addrPrefix}valoperpub`,
            bech32PrefixConsAddr: `${addrPrefix}valcons`,
            bech32PrefixConsPub: `${addrPrefix}valconspub`
        },
        currencies: [currency],
        feeCurrencies: [currency],
        stakeCurrency: currency,
    }

    await window.keplr.experimentalSuggestChain(keplrConfig)
}