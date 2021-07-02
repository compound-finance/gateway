
# Ethereum Cash Token and Starport

## Getting Started

First, you'll need to install the deps, including [Saddle](https://github.com/compound-finance/saddle), which is used for contract management and testing.

```sh
yarn install
```

## Compiling Contracts

You will need `solc` 0.8.1 to be installed and available as `solc`. See solc and saddle documentation for choosing a compiler binary.

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
yarn script deploy -n ropsten 0x2079A734292094702f4D7D64A59e980c20652Cae 300
```

where `0x2079A734292094702f4D7D64A59e980c20652Cae` is the admin, e.g. the Compound Timelock, and `300` is the initial cash yield.

And to upgrade to m3:

```
yarn script deploy:m3 -n ropsten 0x2079A734292094702f4D7D64A59e980c20652Cae
```

And to upgrade to m4:

```
yarn script deploy:m4 -n ropsten 0x2079A734292094702f4D7D64A59e980c20652Cae
```

Goerli:

```
yarn script deploy -n goerli 0xE4a892476d366A1AE55bf53463a367892E885cEE 300
```

## Console

You can connect to a repl for deployed contracts with the following command:

```sh
yarn console -n ropsten

> await starport.methods.cash().call();
```

## License

All contracts are copyright Compound Labs, Inc, 2021 and licensed under GPL-3.0 unless otherwise noted. See `Proxy/LICENSE` for additional licensing.


## Contributing

Please open a pull request, issue or discuss in the Compound governance forums for all changes. Note: the `master` branch of this repo will attempt to match the main-net deployed contracts and not the future development of these contracts.
