const RLP = require('rlp');
const fs = require('fs').promises;
const path = require('path');
const chalk = require('chalk');

const getNextContractAddress = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
};

async function writeJSON(file, data) {
  await fs.writeFile(path.join(__dirname, file), JSON.stringify(data, null, 4));
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

let etherscan = env['etherscan'];

async function deployAndVerify(contract, args, opts) {
  console.log(`Deploying ${chalk.blue(chalk.bold(contract))} with args ${chalk.green(JSON.stringify(args))}`);
  let res = await deploy(contract, args, opts);
  await sleep(10_000); // Give Etherscan time to pick up the contract
  console.log(`Deployed ${contract} to ${res._address} [View on Etherscan](https://${network}.etherscan.io/address/${res._address})`);
  if (etherscan) {
    try {
      console.log(`Verifying ${contract} on Etherscan...`);
      await saddle.verify(etherscan, res._address, contract, args);
    } catch (e) {
      console.log(chalk.yellow(`Failed to verify on Etherscan: ${e}`));
    }
  }
  return res;
}

const main = async (authorityAddresses) => {
  console.log(chalk.yellow(`\n\nDeploying Compound Chain to ${network} with Eth validators: ${authorityAddresses.join(', ')}\n`));

  const root = saddle.account;

  proxyAdmin = await deployAndVerify('ProxyAdmin', [], { from: root });

  const rootNonce = await web3.eth.getTransactionCount(root);
  const cashAddress = getNextContractAddress(root, rootNonce + 4);

  starportImpl = await deployAndVerify('Starport', [cashAddress, root], { from: root });
  proxy = await deployAndVerify('TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: root });
  cashImpl = await deployAndVerify('CashToken', [proxy._address], { from: root });

  starport = await saddle.getContractAt('Starport', proxy._address);
  await starport.methods.changeAuthorities(authorityAddresses).send({ from: root });

  let cashProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    cashImpl._address,
    proxyAdmin._address,
    cashImpl.methods.initialize(0, fromNow(0)).encodeABI()
  ], { from: root });
  cash = await saddle.getContractAt('CashToken', cashProxy._address);

  console.log(`Deployed:
    \t${chalk.bold("Starport")}: ${starport._address}
    \t${chalk.bold("StarportImpl")}: ${starportImpl._address}
    \t${chalk.bold("Cash")}: ${cash._address}
    \t${chalk.bold("CashImpl")}: ${cashImpl._address}
    \t${chalk.bold("ProxyAdmin")}: ${proxyAdmin._address}
  `);

  await writeJSON(`networks/${network}.json`, {
    Contracts: {
      starport: starport._address,
      cash: cash._address,
      starportImpl: starportImpl._address,
      cashImpl: cashImpl._address,
      proxyAdmin: proxyAdmin._address
    }
  });

  await writeJSON(
    `networks/${network}-abi.json`,
    {
      starport: starportImpl._jsonInterface,
      cash: cashImpl._jsonInterface,
      starportImpl: starportImpl._jsonInterface,
      cashImpl: cashImpl._jsonInterface,
      starportProxy: starportProxy._jsonInterface,
      cashProxy: cashProxy._jsonInterface,
      proxyAdmin: proxyAdmin._jsonInterface,
    }
  );
};

(async () => {
  let authorityAddressesRaw = args[0];
  let authorityAddresses = [];
  if (!!authorityAddressesRaw) {
    authorityAddresses = authorityAddressesRaw.split(",").map((address) => {
      if (address.startsWith("0x") && address.length == "42") {
        return address;
      } else {
        throw new Error(`Unknown authority address: ${address}`);
      }
    });
  }

  await main(authorityAddresses);
})();
