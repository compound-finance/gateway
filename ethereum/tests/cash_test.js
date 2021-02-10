const ABICoder = require("web3-eth-abi");
const {
  bigInt,
  e18,
  getNextContractAddress,
  nRandomWallets,
  nRandomAuthorities,
  replaceByte,
  sign,
  signAll,
  ETH_HEADER,
  ETH_ADDRESS
} = require('./utils');

// TODO: test fee token
describe('Starport', () => {
  let starport;
  let cash;
  let [root, account1, account2] = saddle.accounts;

  const authorityWallets = nRandomWallets(3);
  const authorityAddresses = authorityWallets.map(acct => acct.address);


  beforeEach(async () => {
    const rootNonce = await web3.eth.getTransactionCount(root);
    const cashAddress = getNextContractAddress(root, rootNonce + 1);

    starport = await deploy('StarportHarness', [cashAddress, authorityAddresses]);
    cash = await deploy('CashToken', [starport._address]);
  });

  describe('Unit Tests', () => {
    it('should have correct references', async () => {
      expect(await call(starport, 'cash')).toMatchAddress(cash._address);
      expect(await call(cash, 'admin')).toMatchAddress(starport._address);
    });
  });

});
