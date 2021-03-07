const chalk = require('chalk');
const {
  deployAndVerify,
  getNextContractAddress,
  checkAddress,
  fromNow,
  saveNetwork,
} = require('../utils/deploy_utils');

const main = async (admin) => {
  console.log(chalk.yellow(`\n\nDeploying Gateway to ${network} with Admin ${admin}\n`));

  const root = saddle.account;

  proxyAdmin = await deployAndVerify('ProxyAdmin', [], { from: root }, saddle, env, network);

  const rootNonce = await web3.eth.getTransactionCount(root);
  const cashAddress = getNextContractAddress(root, rootNonce + 3, saddle.web3);

  starportImpl = await deployAndVerify('Starport', [cashAddress, root], { from: root }, saddle, env, network);
  starportProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    starportImpl._address,
    proxyAdmin._address,
    "0x"
  ], { from: root }, saddle, env, network);
  starport = await saddle.getContractAt('Starport', starportProxy._address);

  cashImpl = await deployAndVerify('CashToken', [starportProxy._address], { from: root }, saddle, env, network);
  let cashProxy = await deployAndVerify('TransparentUpgradeableProxy', [
    cashImpl._address,
    proxyAdmin._address,
    cashImpl.methods.initialize(0, fromNow(0)).encodeABI()
  ], { from: root }, saddle, env, network);
  cash = await saddle.getContractAt('CashToken', cashProxy._address);

  if (cash._address.toLowerCase() !== cashAddress.toLowerCase()) {
    throw new Error(`Cash address mismatched from expectation: ${cash._address} v ${cashAddress}`);
  }

  await saveNetwork({
    Starport: starport,
    Cash: cash,
    StarportImpl: starportImpl,
    CashImpl: cashImpl,
    ProxyAdmin: proxyAdmin
  }, saddle, env, network);
};

(async () => {
  let [admin] = args;

  checkAddress(admin);

  await main(admin);
})();
