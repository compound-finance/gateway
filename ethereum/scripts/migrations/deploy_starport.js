const chalk = require('chalk');
const {
  deployAndVerify,
  checkAddress,
  readNetwork,
  saveNetwork,
} = require('../utils/deploy_utils');

const main = async (admin) => {
  console.log(chalk.yellow(`\n\nDeploying new Starport Admin to ${network}\n`));

  const root = saddle.account;

  let { Contracts } = await readNetwork(saddle, env, network);
  let cashAddress = Contracts['Cash'];
  if (!cashAddress) {
    throw new Error(`Missing Cash address for network ${network}`);
  }

  starportImpl = await deployAndVerify('Starport', [cashAddress, admin], { from: root }, saddle, env, network);

  if ((await starportImpl.methods.admin().call()).toLowerCase() != admin.toLowerCase()) {
    throw new Error(`Invalid Admin on New Starport Impl`)
  }

  await saveNetwork({
    StarportImpl: starportImpl
  }, saddle, env, network);

  console.log(chalk.yellow(`\n\nNote: you will need to manually upgrade the Starport delegator to use the new StarportImpl\n`));

  /* E.g.
    await this.proxyAdmin.methods.upgradeAndCall(
      starport._address,
      "0xaa39fd81E66Eb9DbEEf3253319516A7317829Eb0",
      "0x"
    ).send();
  */
};

(async () => {
  let [admin, initialYield_] = args;
  checkAddress(admin);
  await main(admin);
})();
