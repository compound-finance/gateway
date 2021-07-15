# Gateway

An interest-bearing stablecoin bridge between all DeFi chains.

Gateway is built on [Substrate](https://substrate.dev).

## Local Development

Follow these steps to prepare a local Substrate development environment :hammer_and_wrench:

### Simple Setup

Install all the required dependencies with a single command (be patient, this can take up to 30
minutes).

```bash
curl https://getsubstrate.io -sSf | bash -s -- --fast
```

### Manual Setup

Find manual setup instructions at the
[Substrate Developer Hub](https://substrate.dev/docs/en/knowledgebase/getting-started/#manual-installation).

### Build

Once the development environment is set up, build Gateway. This command will build the
[Wasm](https://substrate.dev/docs/en/knowledgebase/advanced/executor#wasm-execution) and
[native](https://substrate.dev/docs/en/knowledgebase/advanced/executor#native-execution) code:

```bash
cargo build --release
```

Note: that we require the rust `nightly` toolchain as we rely on unstable features (notably `const_generics`). You should make `nightly` your default rust toolchain.

## Test

To test, run:

```sh
cargo test -- -Z unstable-options --test-threads 1
```

## Run

### Single Node Development Chain

Purge any existing dev chain state:

```bash
./target/release/gateway purge-chain --dev
```

If all else fails the chain state can be purged (on Mac OS X) like:

```bash
rm -rf ~/Library/Application\ Support/gateway/chains/dev
```

Start a dev chain:

```bash
./target/release/gateway --dev
```

Or, start a dev chain with detailed logging:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/gateway -lruntime=debug --dev
```

### Multi-Node Local Testnet

To see the multi-node consensus algorithm in action, run a local testnet with two validator nodes,
Alice and Bob, that have been [configured](./node/src/chain_spec.rs) as the initial
authorities of the `local` testnet chain and endowed with testnet units.

Note: this will require two terminal sessions (one for each node).

Start Alice's node first. The command below uses the default TCP port (30333) and specifies
`/tmp/alice` as the chain database location. Alice's node ID will be
`12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp` (legacy representation:
`QmRpheLN4JWdAnY7HGJfWFNbfkQCb6tFf4vvA6hgjMZKrR`); this is determined by the `node-key`.

```bash
cargo +nightly run -- \
  --db=ParityDB \
  --base-path /tmp/alice \
  --chain=local \
  --alice \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url 'ws://telemetry.polkadot.io:1024 0' \
  --validator
```

In another terminal, use the following command to start Bob's node on a different TCP port (30334)
and with a chain database location of `/tmp/bob`. The `--bootnodes` option will connect his node to
Alice's on TCP port 30333:

```bash
cargo +nightly run -- \
  --db=ParityDB \
  --base-path /tmp/bob \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp \
  --chain=local \
  --bob \
  --port 30334 \
  --ws-port 9945 \
  --telemetry-url 'ws://telemetry.polkadot.io:1024 0' \
  --validator
```

Execute `cargo +nightly run -- --help` to learn more about the Gateway's CLI options.

## Gateway Structure

A Substrate project such as this consists of a number of components that are spread across a few
directories.

### Node

A blockchain node is an application that allows users to participate in a blockchain network.
Substrate-based blockchain nodes expose a number of capabilities:

-   Networking: Substrate nodes use the [`libp2p`](https://libp2p.io/) networking stack to allow the
    nodes in the network to communicate with one another.
-   Consensus: Blockchains must have a way to come to
    [consensus](https://substrate.dev/docs/en/knowledgebase/advanced/consensus) on the state of the
    network. Substrate makes it possible to supply custom consensus engines and also ships with
    several consensus mechanisms that have been built on top of
    [Web3 Foundation research](https://research.web3.foundation/en/latest/polkadot/NPoS/index.html).
-   RPC Server: A remote procedure call (RPC) server is used to interact with Substrate nodes.

There are several files in the `node` directory - take special note of the following:

-   [`chain_spec.rs`](./node/src/chain_spec.rs): A
    [chain specification](https://substrate.dev/docs/en/knowledgebase/integrate/chain-spec) is a
    source code file that defines a Substrate chain's initial (genesis) state. Chain specifications
    are useful for development and testing, and critical when architecting the launch of a
    production chain. Take note of the `development_config` and `testnet_genesis` functions, which
    are used to define the genesis state for the local development chain configuration. These
    functions identify some
    [well-known accounts](https://substrate.dev/docs/en/knowledgebase/integrate/subkey#well-known-keys)
    and use them to configure the blockchain's initial state.
-   [`service.rs`](./node/src/service.rs): This file defines the node implementation. Take note of
    the libraries that this file imports and the names of the functions it invokes. In particular,
    there are references to consensus-related topics, such as the
    [longest chain rule](https://substrate.dev/docs/en/knowledgebase/advanced/consensus#longest-chain-rule),
    the [Aura](https://substrate.dev/docs/en/knowledgebase/advanced/consensus#aura) block authoring
    mechanism and the
    [GRANDPA](https://substrate.dev/docs/en/knowledgebase/advanced/consensus#grandpa) finality
    gadget.

After the node has been [built](#build), refer to the embedded documentation to learn more about the
capabilities and configuration parameters that it exposes:

```shell
./target/release/gateway --help
```

### Runtime

In Substrate, the terms
"[runtime](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary#runtime)" and
"[state transition function](https://substrate.dev/docs/en/knowledgebase/getting-started/glossary#stf-state-transition-function)"
are analogous - they refer to the core logic of the blockchain that is responsible for validating
blocks and executing the state changes they define. The Substrate project in this repository uses
the [FRAME](https://substrate.dev/docs/en/knowledgebase/runtime/frame) framework to construct a
blockchain runtime. FRAME allows runtime developers to declare domain-specific logic in modules
called "pallets". At the heart of FRAME is a helpful
[macro language](https://substrate.dev/docs/en/knowledgebase/runtime/macros) that makes it easy to
create pallets and flexibly compose them to create blockchains that can address
[a variety of needs](https://www.substrate.io/substrate-users/).

Review the [FRAME runtime implementation](./runtime/src/lib.rs) included in Gateway and note
the following:

-   This file configures several pallets to include in the runtime. Each pallet configuration is
    defined by a code block that begins with `impl $PALLET_NAME::Config for Runtime`.
-   The pallets are composed into a single runtime by way of the
    [`construct_runtime!`](https://crates.parity.io/frame_support/macro.construct_runtime.html)
    macro, which is part of the core
    [FRAME Support](https://substrate.dev/docs/en/knowledgebase/runtime/frame#support-library)
    library.

### Pallets

The runtime in this project is constructed using many FRAME pallets that ship with the
[core Substrate repository](https://github.com/paritytech/substrate/tree/master/frame) and a
CASH pallet that is [defined in the `pallets`](./pallets/cash/src/lib.rs) directory.

A FRAME pallet is compromised of a number of blockchain primitives:

-   Storage: FRAME defines a rich set of powerful
    [storage abstractions](https://substrate.dev/docs/en/knowledgebase/runtime/storage) that makes
    it easy to use Substrate's efficient key-value database to manage the evolving state of a
    blockchain.
-   Dispatchables: FRAME pallets define special types of functions that can be invoked (dispatched)
    from outside of the runtime in order to update its state.
-   Events: Substrate uses [events](https://substrate.dev/docs/en/knowledgebase/runtime/events) to
    notify users of important changes in the runtime.
-   Errors: When a dispatchable fails, it returns an error.
-   Config: The `Config` configuration interface is used to define the types and parameters upon which
    a FRAME pallet depends.

### Run in Docker

First, install [Docker](https://docs.docker.com/get-docker/).

Then build the Dockerfile:

```sh
docker build -t gateway .
```

Next, start your chain:

```sh
docker run --rm -it gateway -- /bin/sh
```

## Release Process

All upgrades to Gateway should happen via the release process.
We need to track which features were included in the release, usually including a changelog that covers each PR that was merged.

Proper release management is important here, especially since releases include many varieties of breaking changes.
A scenario test should be written to show that things work as expected after the upgrade takes place.
The goal is to not break things on release.

Releases should be cut from the `develop` (default) branch on [Github](https://github.com/compound-finance/gateway).

### Bump Spec Version

First increment the spec version in `runtime/src/lib.rs`.

### Build Your Release Version

```sh
scripts/build_release.sh m88 # replace with your milestone version
```

### Update the Dockerfile

Replace the release in the Dockerfile, using your tag (e.g. here `m88`):

```diff
- RUN scripts/pull_release.sh m16
- RUN chmod +x releases/m16/gateway-linux-x86
+ RUN scripts/pull_release.sh m88
+ RUN chmod +x releases/m88/gateway-linux-x86
```

### Update Chain Spec

Note: this is *only* necessary if you are deploying a new chain, the chain spec is defined at genesis.

```
$ gateway> chains/build_spec.js -r m88 -c stablenet # replace m88 with your version
```

### Using Github Workflows Release Process

Github actions have been created to respond to git tags that are pushed to the repo that begin with `m`, followed by the spec version, e.g. `m88`.

This process will create a draft / pre-release that can be modified and published on github once ready.

Create a tag and push to this repo:

```sh
$ git tag -a m88 # use your milestone tag
$ git push origin m88
```

### Contributing

Contributors are welcome, and will be held to a high standard. Please consider making an issue to discuss larger changes before making pull requests. All contributions will fall under the license on this repo, which **currently does not grant open permission of use**. Additionally, **you agree that your code will be subject to any license which is later attached to this repository as if it had been initially licensed thusly**.
