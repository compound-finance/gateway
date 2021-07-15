
## Chains

This folder contains information about known Gateway chains and provides the ability to generate chain specs.

## Building a new spec

To build a new spec on <stablenet> with release <m16> from `chains/stablenet/chain-config.json` which will generate `chains/stablenet/chain-spec.json` and `chains/stablenet/chain-spec-raw.json`, run:

```sh
./build_spec.js -c stablenet -r m16
```

## Repl

To connect to a chain via the REPL, run:

```sh
yarn install
```

Then:

```sh
yarn repl -c testnet
```

Or:

```sh
yarn repl -c stablenet
```

This will connect you to a REPL to testnet, including to the Ethereum contracts.

### Repl Examples

#### API Query

Query the current cash yield

```sh
> (await api.query.cash.cashYield()).toJSON()
300
```

#### `.block` command

Show the current test-net block

```sh
> .block
#46739
```

#### `.validators` command

Show the current test-net validators

```sh
> .validators
  5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL:
    substrate_id=0x1cbd2d43530a44705ad088af313e18f80b53ef16b36177cd4b77b846f2a5f07c
    eth_address=0x286f2a10c28c966e97a72b8246041fbf636e673e

  5HMqNs9offzpiebeuzHVwKcJRid1L44KgEKF7jwpYYFM25kY:
    substrate_id=0xea3da7e5b24ee22ce5fa252136745bfbefcb657201404f4479bcbe42135e234c
    eth_address=0x4515e1ce5d4c42da4b0561f52ef12dee19f9c020

  5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty:
    substrate_id=0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48
    eth_address=0xc9b0c3ed4efa833a7ad5459755a18f9689a0f7ac

  5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY:
    substrate_id=0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
    eth_address=0x55413a2d4908d130c908ccf2f298b235bacd427a
```

#### Ethereum Contract

Show the Starport admin:

```sh
> await starport.methods.admin().call()
'0x046231a12d30248bAD3322Af74CEA9c325627D32'
```
