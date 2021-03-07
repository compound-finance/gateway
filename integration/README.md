
## Integration Tests

Integration tests that run the real deal and make sure every component works together. The goal of these tests are to show that end-to-end flows are correctly working. Specifically, we will run ganache, deploy Cash Token, Starport, etc, run a simplified Gateway test-net and then interact with ganache and Gateway to say mint some Cash or extract collateral from Gateway. These are fully automated and run in CI.

## Running

First, you'll need to compile Gateway (in release mode) and compile the Ethereum contracts.

```sh
gateway> cargo +nightly build
```

Note: if you require deeper debugging, you may want to enable the `runtime-debug` feature, via:

```sh
gateway> cargo +nightly build --features "runtime-debug"
```

This will remove `wasm-stripped` messages at the cost of a larger wasm runtime blob. This should not be used for production builds.

In the `ethereum` directory, run:

Note: you'll need solc 0.7.5 installed

```sh
gateway/ethereum> yarn install && yarn compile
```

Next, install integration test dependencies in this directory:

```sh
gateway/integration> yarn install
```

Next, run the test suite or a single chosen test-case:

```sh
yarn test
```

```sh
yarn test __tests__/golden_test.js
```
