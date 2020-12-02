const RLP = require('rlp');

const getHypotheticalAddr = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
}

const main = async () => {
  const deployer = saddle.account;
  const nonce = await web3.eth.getTransactionCount(deployer);
  const starportAddr = getHypotheticalAddr(deployer, nonce);
  const cashAddr = getHypotheticalAddr(deployer, nonce + 1);

  const starport = await deploy('Starport', [cashAddr, args]);
  const cash = await deploy('MockCashToken', [starportAddr, BigInt(1e18).toString(), deployer]);
  console.log("DEPLOYED STARPORT TO: ", starport._address, "DEPLOYED CASH TO: ", cash._address);
};


(async () => {
  await main();
})();
