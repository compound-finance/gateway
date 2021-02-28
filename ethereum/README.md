
# Ethereum Cash Token and Starport

## Getting Started

First, you'll need to install the deps, including [Saddle](https://github.com/compound-finance/saddle), which is used for contract management and testing.

```sh
yarn install
```

## Compiling Contracts

You will need `solc` 8.0.1 to be installed and available as `solc`. See solc and saddle documentation for choosing a compiler binary.

```sh
yarn compile
```

This will compile your contracts to the `.build` directory.

## Testing Contracts

To test your contracts via Jest from saddle, run:

```sh
yarn test
```

## Deployment

You can deploy the Starport with the following command:

```
npx saddle script -n ropsten deploy {0xAuthority0,0xAuthority1,...}
```

## Console

You can connect to a repl for deployed contracts with the following command:

```sh
yarn console -n goerli

> await starport.methods.cash().call();
```

## License

All contracts are copyright Compound Labs, Inc, 2021 and licensed under GPL-3.0 unless otherwise noted. See `Proxy/LICENSE` for additional licensing.


## Contributing

Please open a pull request, issue or discuss in the Compound governance forums for all changes. Note: the `master` branch of this repo will attempt to match the main-net deployed contracts and not the future development of these contracts.
