const chalk = require('chalk');
const {
  deployAndVerify,
  checkAddress,
  readNetwork,
  saveNetwork,
} = require('../utils/deploy_utils');

const NEW_LOCK_EVENT_TOPIC = '0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386';

const main = async () => {
  console.log(chalk.yellow(`\n\nDeploying Gateway m3 to ${network}\n`));

  const root = saddle.account;

  let { Contracts } = await readNetwork(saddle, env, network);
  let cashAddress = Contracts['Cash'];
  if (!cashAddress) {
    throw new Error(`Missing Cash address for network ${network}`);
  }

  starportImpl = await deployAndVerify('Starport', [cashAddress, root], { from: root }, saddle, env, network);

  let lockEventTopic = starportImpl.events.Lock().arguments[0].topics[0];
  if (lockEventTopic !== NEW_LOCK_EVENT_TOPIC) {
    throw new Error(`Invalid new event lock topic: ${lockEventTopic} v ${NEW_LOCK_EVENT_TOPIC}. Have you compiled recent contracts?`);
  } 

  await saveNetwork({
    StarportImpl: starportImpl
  }, saddle, env, network);

  console.log(chalk.yellow(`\n\nNote: you will need to manually upgrade the Starport delegator to use the new StarportImpl\n`));
};

(async () => {
  await main();
})();
