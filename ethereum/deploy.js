const RLP = require('rlp');
const fs = require('fs').promises;
const path = require('path');

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
  console.log(`Deploying ${contract} with args ${JSON.stringify(args)}`);
  let res = await deploy(contract, args, opts);
  await sleep(10_000); // Give Etherscan time to pick up the contract
  console.log(`Deployed ${contract} to ${res._address} [View on Etherscan](https://${network}.etherscan.io/address/${res._address})`);
  if (etherscan) {
    try {
      console.log(`Verifying ${contract} on Etherscan...`);
      await saddle.verify(etherscan, res._address, contract, args);
    } catch (e) {
      console.log(`Failed to verify on Etherscan: ${e}`);
    }
  }
  return res;
}

const main = async () => {
  const root = saddle.account;

  proxyAdmin = await deployAndVerify('ProxyAdmin', [], { from: root });

  const rootNonce = await web3.eth.getTransactionCount(root);
  const cashAddress = getNextContractAddress(root, rootNonce + 4);

  starportImpl = await deployAndVerify('StarportHarness', [cashAddress, root], { from: root });
  proxy = await deployAndVerify('TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: root });
  cashImpl = await deployAndVerify('CashToken', [proxy._address], { from: root });

  starport = await saddle.getContractAt('StarportHarness', proxy._address);
  await starport.methods.changeAuthorities(authorityAddresses).send({ from: root });

  let cashProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    cashImpl._address,
    proxyAdmin._address,
    cashImpl.methods.initialize(0, fromNow(0)).encodeABI()
  ], { from: root });
  cash = await saddle.getContractAt('CashToken', cashProxy._address);

  console.log(`Deployed:
    \tStarport: ${starport._address}
    \tStarportImpl: ${starportImpl._address}
    \tCash: ${cash._address}
    \tCashImpl: ${cashImpl._address}
    \tProxyAdmin: ${proxyAdmin._address}
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
  await main();
})();
