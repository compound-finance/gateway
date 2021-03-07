const chalk = require('chalk');
const {
  deployAndVerify,
  checkAddress,
  readNetwork,
  saveNetwork,
} = require('../utils/deploy_utils');

const TEST_SIGNATURE = '0xc86ba4a27a0938efd58b397cf49e453e4abb3f3283acc64644a419c7734ede3c6a5704ded16ff1852340925ee6149c8fe76e9f1610af8c73bcf9b8d28ef086fd1c';

const main = async () => {
  console.log(chalk.yellow(`\n\nDeploying Gateway m4 to ${network}\n`));

  const root = saddle.account;

  let { Contracts } = await readNetwork(saddle, env, network);
  let cashAddress = Contracts['Cash'];
  if (!cashAddress) {
    throw new Error(`Missing Cash address for network ${network}`);
  }

  starportImpl = await deployAndVerify('Starport', [cashAddress, root], { from: root }, saddle, env, network);

  let starportAddress = Contracts['Starport'];
  if (!starportAddress) {
    throw new Error(`Missing Starport address for network ${network}`);
  }
  cashImpl = await deployAndVerify('CashToken', [starportAddress], { from: root }, saddle, env, network);

  if (await cashImpl.methods.symbol().call() !== "CASH") {
    throw new Error(`Invalid CASH Token. Have you compiled recent contracts?`);
  }

  let error;
  try {
    await starportImpl.methods.invoke("0x", [TEST_SIGNATURE]).call()
  } catch (e) {
    error = e;
  }
  if (!error.message.includes('Below quorum threshold')) {
    throw new Error(`Invalid Starport (expected quorum threshold error, got "${error.message}"). Have you compiled recent contracts?`);
  }

  await saveNetwork({
    StarportImpl: starportImpl
    CashImpl: cashImpl
  }, saddle, env, network);

  console.log(chalk.yellow(`\n\nNote: you will need to manually upgrade the Starport and Cash delegator to use the new StarportImpl and CashImpl\n`));
};

(async () => {
  await main();
})();
