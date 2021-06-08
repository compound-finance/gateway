
## Integration Tests

Integration tests that run the real deal and make sure every component works together. The goal of these tests are to show that end-to-end flows are correctly working. Specifically, we will run ganache, deploy Cash Token, Starport, etc, run a simplified Gateway test-net and then interact with ganache and Gateway to say mint some Cash or extract collateral from Gateway. These are fully automated and run in CI.

## Building

You can build the Starport code and Gateway node using:

```sh
gateway/integration> yarn && yarn build
```

This will build the chain using `release` mode, and special flags used for integration testing.

## Running

Running the entire test suite doesn't really work right now so run one test at a time by using `only:true` on the test you want to run as follows:

```
buildScenarios('...', scen_info, [
  {
    name: '...',
    only: true,
    scenario: async ({ ashley, usdc, chain, ... }) => {
        .
        .
        .
    }
  },
  .
  .
  .
]);
```

Then run the test using:

```sh
yarn test __tests__/<my-test>.js
```
