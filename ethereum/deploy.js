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

function checkAddress(address) {
  if (address.startsWith("0x") && address.length == "42") {
    return;
  } else {
    throw new Error(`Invalid address: ${address}`);
  }
}

const fromNow = (seconds) => {
  return Math.floor(seconds + (new Date() / 1000));
}

let etherscan = env['etherscan'];

// Delay for Etherscan to pick up contract
let etherscanDelay = env['etherscan_delay'] ? Number(env['etherscan_delay']) : 25_000;

async function deployAndVerify(contract, args, opts) {
  console.log(`Deploying ${chalk.blue(chalk.bold(contract))} with args ${chalk.green(JSON.stringify(args))}`);
  let res = await deploy(contract, args, opts);
  console.log(`Deployed ${contract} to ${res._address} [View on Etherscan](https://${network}.etherscan.io/address/${res._address})\n`);
  if (etherscan && network !== 'development') {
    await sleep(etherscanDelay); // Give Etherscan time to pick up the contract

    try {
      console.log(`Verifying ${contract} on Etherscan...`);
      await saddle.verify(etherscan, res._address, contract, args);
    } catch (e) {
      console.log(chalk.yellow(`Failed to verify on Etherscan: ${e}`));
    }
  }
  return res;
}

const main = async (admin) => {
  console.log(chalk.yellow(`\n\nDeploying Compound Chain to ${network} with Admin ${admin}\n`));

  const root = saddle.account;

  proxyAdmin = await deployAndVerify('ProxyAdmin', [], { from: root });

  const rootNonce = await web3.eth.getTransactionCount(root);
  const cashAddress = getNextContractAddress(root, rootNonce + 3);

  starportImpl = await deployAndVerify('Starport', [cashAddress, root], { from: root });
  starportProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: root });
  starport = await saddle.getContractAt('Starport', starportProxy._address);

  cashImpl = await deployAndVerify('CashToken', [starportProxy._address], { from: root });
  let cashProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    cashImpl._address,
    proxyAdmin._address,
    cashImpl.methods.initialize(0, fromNow(0)).encodeABI()
  ], { from: root });
  cash = await saddle.getContractAt('CashToken', cashProxy._address);

  if (cash._address.toLowerCase() !== cashAddress.toLowerCase()) {
    throw new Error(`Cash address mismatched from expectation: ${cash._address} v ${cashAddress}`);
  }

  console.log(`\n${chalk.blue("Deployed")}:
    \t${chalk.bold("Starport")}: ${starport._address}
    \t${chalk.bold("StarportImpl")}: ${starportImpl._address}
    \t${chalk.bold("Cash")}: ${cash._address}
    \t${chalk.bold("CashImpl")}: ${cashImpl._address}
    \t${chalk.bold("ProxyAdmin")}: ${proxyAdmin._address}
  `);

  await writeJSON(`networks/${network}.json`, {
    Contracts: {
      Starport: starport._address,
      Cash: cash._address,
      StarportImpl: starportImpl._address,
      CashImpl: cashImpl._address,
      ProxyAdmin: proxyAdmin._address
    }
  });

  await writeJSON(
    `networks/${network}-abi.json`,
    {
      Starport: starportImpl._jsonInterface,
      Cash: cashImpl._jsonInterface,
      StarportImpl: starportImpl._jsonInterface,
      CashImpl: cashImpl._jsonInterface,
      StarportProxy: starportProxy._jsonInterface,
      CashProxy: cashProxy._jsonInterface,
      ProxyAdmin: proxyAdmin._jsonInterface,
    }
  );
};

(async () => {
  let [admin] = args;

  checkAddress(admin);

  await main(admin);
})();
