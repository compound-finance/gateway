const { expect } = require("chai");
const {
  e18,
  fromNow,
  sendRPC,
  ETH_ZERO_ADDRESS
} = require('./utils');

describe.skip('CashToken', () => {
  let proxyAdmin;
  let cashImpl;
  let proxy;
  let cash;
  let root, admin, account1, account2, account3;

  let startCashIndex = e18(1);
  let start = fromNow(0);

  // Due the transparent proxy, this is the way to read `implementation` and `admin` when calling not as the admin
  // Note: we could call as the admin, but it's important to know how to read this way, anyway.
  async function getProxyImplementation(contract) {
    return await web3.eth.getStorageAt(contract._address, '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc');
  }

  async function getProxyAdmin(contract) {
    return await web3.eth.getStorageAt(contract._address, '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103');
  }

  beforeEach(async () => {
    [root, admin, account1, account2, account3] = await ethers.getSigners();
    proxyAdmin = await deploy('ProxyAdmin', [], { from: root });
    cashImpl = await deploy('CashToken', [admin], { from: root });
    proxy = await deploy('TransparentUpgradeableProxy', [
      cashImpl._address,
      proxyAdmin._address,
      cashImpl.methods.initialize(0, start).encodeABI()
    ], { from: root });
    cash = await saddle.getContractAt('CashToken', proxy._address);
  });

  describe('#constructor', () => {
    it('should have correct admin and yield references', async () => {
      expect(await call(cash, 'admin')).toMatchAddress(admin);
      let cashYieldAndIndex = await call(cash, 'cashYieldAndIndex');
      let cashYieldStart = await call(cash, 'cashYieldStart');
      let initialized = await call(cash, 'initialized');
      expect(cashYieldAndIndex.index).toEqualNumber(1e18);
      expect(cashYieldAndIndex.yield).toEqualNumber(0);
      expect(cashYieldStart).toEqualNumber(start);
      expect(initialized).toEqual(true);
    });

    it('should have correct admin and yield references when non-zero', async () => {
      let proxyAdmin = await deploy('ProxyAdmin', [], { from: root });
      let cashImpl = await deploy('CashToken', [admin], { from: root });
      let proxy = await deploy('TransparentUpgradeableProxy', [
        cashImpl._address,
        proxyAdmin._address,
        cashImpl.methods.initialize(500, start).encodeABI()
      ], { from: root });
      let cash = await saddle.getContractAt('CashToken', proxy._address);
      let cashYieldAndIndex = await call(cash, 'cashYieldAndIndex');
      expect(cashYieldAndIndex.index).toEqualNumber(1e18);
      expect(cashYieldAndIndex.yield).toEqualNumber(500);
    });

    it('should not allow initialize to be called twice', async () => {
      await expect(cash.methods.initialize(500, start).call()).rejects.toRevert("revert Cash Token already initialized");
    });

    it('should fail if not initialized', async () => {
      let proxyAdmin = await deploy('ProxyAdmin', [], { from: root });
      let cashImpl = await deploy('CashToken', [admin], { from: root });
      let proxy = await deploy('TransparentUpgradeableProxy', [
        cashImpl._address,
        proxyAdmin._address,
        "0x"
      ], { from: root });
      let cash = await saddle.getContractAt('CashToken', proxy._address);
      await expect(call(cash, 'getCashIndex')).rejects.toRevert("revert Cash Token uninitialized");
    });
  });

  describe('Upgradeable', () => {
    it('should be upgradeable to new logic (without call)', async () => {
      expect(await getProxyImplementation(cash)).toMatchAddress(cashImpl._address);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      let cashImpl2 = await deploy('CashToken2', [account3], { from: root });
      await proxyAdmin.methods.upgrade(cash._address, cashImpl2._address).send({ from: root });

      expect(await getProxyImplementation(cash)).toMatchAddress(cashImpl2._address);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);

      expect(await cash.methods.admin().call()).toEqual(account3);

      // We'll need this for new function calls
      cash = await saddle.getContractAt('CashToken2', proxy._address);
      await cash.methods.count_().send();
      expect(await cash.methods.counter().call()).toEqualNumber(1);
    });

    it('should only be upgradeable by the admin', async () => {
      let cashImpl2 = await deploy('CashToken2', [admin], { from: root });
      await expect(proxyAdmin.methods.upgrade(cash._address, cashImpl2._address).send({ from: account1 })).rejects.toRevert('revert Ownable: caller is not the owner');
      expect(await getProxyImplementation(cash)).toMatchAddress(cashImpl._address);
    });

    it('should allow initialization during upgrade', async () => {
      let cashImpl2 = await deploy('CashToken2', [admin], { from: root });
      await proxyAdmin.methods.upgradeAndCall(
        cash._address,
        cashImpl2._address,
        cashImpl2.methods.initialize_(10).encodeABI()
      ).send({ from: root });
      expect(await getProxyImplementation(cash)).toMatchAddress(cashImpl2._address);
      cash = await saddle.getContractAt('CashToken2', proxy._address);
      expect(await cash.methods.counter().call()).toEqualNumber(10);
    });

    it('should not allow re-initialization during upgrade', async () => {
      let cashImpl2 = await deploy('CashToken2', [admin], { from: root });
      await proxyAdmin.methods.upgradeAndCall(
        cash._address,
        cashImpl2._address,
        cashImpl2.methods.initialize_(10).encodeABI()
      ).send({ from: root });
      cash = await saddle.getContractAt('CashToken2', proxy._address);
      await cash.methods.count_().send();
      await expect(cash.methods.initialize_(100).call()).rejects.toRevert("revert cannot reinitialize");
      expect(await cash.methods.counter().call()).toEqualNumber(11);
    });

    it('should be able to rotate proxy admin', async () => {
      expect(await proxyAdmin.methods.getProxyAdmin(cash._address).call()).toMatchAddress(proxyAdmin._address);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      expect(await cash.methods.admin().call()).toMatchAddress(admin);
      await proxyAdmin.methods.changeProxyAdmin(cash._address, account1).send({ from: root });
      expect(await getProxyAdmin(cash)).toMatchAddress(account1);
      expect(await cash.methods.admin().call()).toMatchAddress(admin);
    });

    it('should not allow rotation unless admin', async () => {
      expect(await proxyAdmin.methods.getProxyAdmin(cash._address).call()).toMatchAddress(proxyAdmin._address);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      expect(await cash.methods.admin().call()).toMatchAddress(admin);
      await expect(proxyAdmin.methods.changeProxyAdmin(cash._address, account1).send({ from: account1 })).rejects.toRevert("revert Ownable: caller is not the owner");
      expect(await proxyAdmin.methods.getProxyAdmin(cash._address).call()).toMatchAddress(proxyAdmin._address);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      expect(await cash.methods.admin().call()).toMatchAddress(admin);
    });

    it('should be able to rotate proxy admin\'s admin', async () => {
      expect(await proxyAdmin.methods.owner().call()).toMatchAddress(root);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      await proxyAdmin.methods.transferOwnership(account1).send({ from: root });
      expect(await proxyAdmin.methods.owner().call()).toMatchAddress(account1);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
    });

    it('should not be able to rotate proxy admin\'s admin unless from current admin', async () => {
      expect(await proxyAdmin.methods.owner().call()).toMatchAddress(root);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
      await expect(proxyAdmin.methods.transferOwnership(account1).send({ from: account1 })).rejects.toRevert("revert Ownable: caller is not the owner");;
      expect(await proxyAdmin.methods.owner().call()).toMatchAddress(root);
      expect(await getProxyAdmin(cash)).toMatchAddress(proxyAdmin._address);
    });
  });

  describe('#setFutureYield', () => {
    it('should set correct current and next indexes, yields and startTimes', async () => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp + 30 * 60;

      const yieldAndIndex_before = await call(cash, 'cashYieldAndIndex');
      const start_before = await call(cash, 'cashYieldStart');

      // Update future yield, first change
      await send(cash, 'setFutureYield', [362, 1e6, nextYieldTimestamp], { from: admin });
      const yieldAndIndex_change = await call(cash, 'cashYieldAndIndex');
      const start_change = await call(cash, 'cashYieldStart');
      const nextYieldAndIndex_change = await call(cash, 'nextCashYieldAndIndex');
      const nextStart_change = await call(cash, 'nextCashYieldStart');

      expect(yieldAndIndex_change.yield).toEqualNumber(yieldAndIndex_before.yield);
      expect(yieldAndIndex_change.index).toEqualNumber(yieldAndIndex_before.index);
      expect(start_change).toEqualNumber(start_before);
      expect(nextYieldAndIndex_change.yield).toEqualNumber(362);
      expect(nextYieldAndIndex_change.index).toEqualNumber(1e6);
      expect(nextStart_change).toEqualNumber(nextYieldTimestamp);

      await sendRPC(web3, "evm_increaseTime", [31 * 60]);

      // Update future yield, second change, current yield, index and time are set to previous next values
      await send(cash, 'setFutureYield', [369, 11e5, nextYieldTimestamp + 60 * 60], { from: admin });
      const yieldAndIndex_change2 = await call(cash, 'cashYieldAndIndex');
      const start_change2 = await call(cash, 'cashYieldStart');
      const nextYieldAndIndex_change2 = await call(cash, 'nextCashYieldAndIndex');
      const nextStart_change2 = await call(cash, 'nextCashYieldStart');

      expect(yieldAndIndex_change2.yield).toEqualNumber(nextYieldAndIndex_change.yield);
      expect(yieldAndIndex_change2.index).toEqualNumber(nextYieldAndIndex_change.index);
      expect(start_change2).toEqualNumber(nextStart_change);
      expect(nextYieldAndIndex_change2.yield).toEqualNumber(369);
      expect(nextYieldAndIndex_change2.index).toEqualNumber(11e5);
      expect(nextStart_change2).toEqualNumber(nextYieldTimestamp + 60 * 60);
    });

    it('should fail if called not by an admin', async() => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp + 30 * 60;
      await expect(send(cash, 'setFutureYield', [300, 1e6, nextYieldTimestamp], { from: account1 })).rejects.toRevert("revert Must be admin");
    });

    it('should fail if next yield start is before current yield start', async() => {
      const start_yield = await call(cash, 'cashYieldStart');
      await expect(send(cash, 'setFutureYield', [300, 1e6, start_yield], { from: admin })).rejects.toRevert("revert Invalid yield start");
      await expect(send(cash, 'setFutureYield', [300, 1e6, start_yield - 1000], { from: admin })).rejects.toRevert("revert Invalid yield start");
    });

    it('should fail if yield range is invalid', async() => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp + 30 * 60;
      await expect(send(cash, 'setFutureYield', [30000, 1e6, nextYieldTimestamp], { from: admin })).rejects.toRevert("revert Invalid yield range");
    });
  });

  describe('#mint', () => {
    it('should mint tokens and emit `Transfer` event', async () => {
      expect(await call(cash, 'totalSupply')).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      const result = principal * cashIndex / 1e18;
      const tx = await send(cash, 'mint', [account1, principal], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber(result);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(result);

      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: ETH_ZERO_ADDRESS,
        to: account1,
        value: result.toString()
      });
    });

    it('should fail if called not by an admin', async() => {
      await expect(send(cash, 'mint', [account1, 10e6], { from: account1 })).rejects.toRevert("revert Must be admin");
    })
  });

  describe('#burn', () => {
    it('should burn tokens and emit `Transfer` event', async () => {
      // Let's mint tokens first, to have something to burn
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      const burnAmount = 5e6 * cashIndex / 1e18;
      await send(cash, 'mint', [account1, principal], { from: admin });

      // An attempt to burn tokens
      const tx = await send(cash, 'burn', [account1, burnAmount], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber(burnAmount);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(burnAmount);

      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: account1,
        to: ETH_ZERO_ADDRESS,
        value: (burnAmount).toString()
      });
    });

    it('should fail if called not by an admin', async() => {
      await expect(send(cash, 'burn', [account1, 10e6], { from: account1 })).rejects.toRevert("revert Must be admin");
    })
  });

  describe('#totalSupply', () => {
    it('should return total supply of cash', async () => {
      expect(await call(cash, 'totalSupply')).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const principal1 = 10e6;
      const principal2 = 5e6;
      await send(cash, 'mint', [account1, principal1], { from: admin });
      await send(cash, 'mint', [account2, principal2], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber((principal1 + principal2) * cashIndex / 1e18);
    });
  });

  describe('#balanceOf', () => {
    it('should return balance of Cash tokens for given account', async () => {
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account2])).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const principal1 = 10e6;
      const principal2 = 5e6;
      await send(cash, 'mint', [account1, principal1], { from: admin });
      await send(cash, 'mint', [account2, principal2], { from: admin });

      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(principal1 * cashIndex / 1e18);
      expect(await call(cash, 'balanceOf', [account2])).toEqualNumber(principal2 * cashIndex / 1e18);
    });
  });

  describe('#name', () => {
    it('should return Cash token name', async () => {
      expect(await call(cash, 'name', [])).toEqual("Cash");
    });
  });

  describe('#symbol', () => {
    it('should return Cash token symbol', async () => {
      expect(await call(cash, 'symbol', [])).toEqual("CASH");
    });
  });

  describe('#decimals', () => {
    it('should return Cash token decimals number', async () => {
      expect(await call(cash, 'decimals', [])).toEqualNumber(6);
    });
  });

  describe('#approve, allowance', () => {
    it('should approve transfers and modify allowances', async () => {
      expect(await call(cash, 'allowance', [account1, account2])).toEqualNumber(0);
      const amount = 10e6;
      const tx = await send(cash, 'approve', [account2, amount], { from: account1});
      expect(await call(cash, 'allowance', [account1, account2])).toEqualNumber(amount);
      expect(tx.events.Approval.returnValues).toMatchObject({
        owner: account1,
        spender: account2,
        value: amount.toString()
      });
    });
  });

  describe('#transfer', () => {
    it('should transfer Cash tokens between users', async() => {
      // Mint tokes first to have something to transfer
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      await send(cash, 'mint', [account1, principal], { from: admin });

      const amount = principal * cashIndex / 1e18;
      const tx = await send(cash, 'transfer', [account2, amount], { from: account1 });
      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: account1,
        to: account2,
        value: amount.toString()
      });

      expect(await call(cash, 'totalSupply')).toEqualNumber(amount);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account2])).toEqualNumber(amount);
    });

    it('should fail if recipient is invalid', async() => {
      await expect(send(cash, 'transfer', [account1, 1e6], { from: account1 })).rejects.toRevert("revert Invalid recipient");
    });

    it('should fail if not enough Cash tokens to transfer', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      await send(cash, 'mint', [account1, principal], { from: admin });

      const amount = principal * cashIndex / 1e18;
      // An attempt to transfer double amount
      await expect(send(cash, 'transfer', [account2, 2 * amount], { from: account1 })).rejects.toRevert("revert");
    });
  });

  describe('#transferFrom', () => {
    it('should transfer Cash tokens between users', async() => {
      // Mint tokes first to have something to transfer
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      await send(cash, 'mint', [account1, principal], { from: admin });
      const amount = principal * cashIndex / 1e18;

      // Approve an account2 to move tokens on behalf of account1
      await send(cash, 'approve', [account2, amount], {from: account1});

      const tx = await send(cash, 'transferFrom', [account1, account3, amount], { from: account2 });
      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: account1,
        to: account3,
        value: amount.toString()
      });

      expect(await call(cash, 'totalSupply')).toEqualNumber(amount);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account3])).toEqualNumber(amount);
    });

    it('should fail if recipient is invalid', async() => {
      await expect(send(cash, 'transferFrom', [account1, account1, 1e6], { from: account1 })).rejects.toRevert("revert Invalid recipient");
    });

    it('should fail if not enough allowance', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      await send(cash, 'mint', [account1, principal], { from: admin });
      const amount = principal * cashIndex / 1e18;

      // Approve an account2 to move tokens on behalf of account1
      await send(cash, 'approve', [account2, amount / 2], {from: account1});

      // An attempt to transfer double the approved amount
      await expect(send(cash, 'transferFrom', [account1, account3, amount], { from: account2 })).rejects.toRevert("revert");
    });

    it('should fail if not enough Cash tokens to transfer', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const principal = 10e6;
      await send(cash, 'mint', [account1, principal], { from: admin });
      const amount = principal * cashIndex / 1e18;

      // Approve an account2 to move tokens on behalf of account1
      await send(cash, 'approve', [account2, 2 * amount], {from: account1});

      // An attempt to transfer double the available amount
      await expect(send(cash, 'transferFrom', [account1, account3, 2 * amount], { from: account2 })).rejects.toRevert("revert");
    });
  });

  describe("#getCashIndex tests", () => {
    it('getCashIndex is growing over time', async() => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp;

      // Set non-zero cash yield
      await send(cash, 'setFutureYield', [300, startCashIndex, nextYieldTimestamp], { from: admin });

      // Cash index after 2 minutes
      await sendRPC(web3, "evm_increaseTime", [2 * 60]);
      await sendRPC(web3, "evm_mine", []);

      const cashIndex1 = await call(cash, 'getCashIndex');
      // expect(cashIndex1).toEqualNumber('1000000114155257656');

      await sendRPC(web3, "evm_increaseTime", [10]);
      await sendRPC(web3, "evm_mine", []);

      // Cash index after 2 minutes + 10 seconds
      const cashIndex2 = await call(cash, 'getCashIndex');
      // expect(cashIndex2).toEqualNumber('1000000123668196382');
      expect(cashIndex2).greaterThan(cashIndex1);

      await sendRPC(web3, "evm_increaseTime", [30 * 60]);
      await sendRPC(web3, "evm_mine", []);

      // Cash index after 2 minutes + 10 seconds + 30 minutes = 1930 seconds
      const cashIndex3 = await call(cash, 'getCashIndex');
      // expect(cashIndex3).toEqualNumber('1000001835998641302');
      expect(cashIndex3).greaterThan(cashIndex2);

      await sendRPC(web3, "evm_increaseTime", [24 * 60 * 60]);
      await sendRPC(web3, "evm_mine", []);

      // Cash index after 2 minutes + 10 seconds + 30 minutes + 1 day = 88330 seconds
      const cashIndex4 = await call(cash, 'getCashIndex');
      // expect(cashIndex4).toEqualNumber('1000084031308210378');
      expect(cashIndex4).greaterThan(cashIndex3);
    });

    it('exponent helper function works', async() => {
      // Exponent for 10 sec time difference
      const seconds_per_year = 365 * 24 * 60 * 60;
      const yield_bps = 300
      const yield_percent = yield_bps / 1e4;

      // Simple sanity check
      const exp1 = await call(cash, 'exponent', [yield_bps, 10]);
      expect(exp1).toEqualNumber('1000000009512937640');

      let time_sec = 1;
      let precision = 10;
      while (time_sec <= seconds_per_year / 4) {
        const exp = await call(cash, 'exponent', [yield_bps, time_sec]);
        const exp_check = Math.exp(yield_percent * time_sec / seconds_per_year) * 1e18;
        expect(exp).toBeWithinRange(exp_check - precision, exp_check + precision);
        time_sec = time_sec * 10;
        precision = precision * 10;
      }
    })

    it('user cash balance is growing', async() => {
      // Set non-zero cash yield
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp;
      await send(cash, 'setFutureYield', [300, startCashIndex, nextYieldTimestamp], { from: admin });

      // Mint cash tokens
      expect(await call(cash, 'totalSupply')).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);
      const principal = 3e6;
      await send(cash, 'mint', [account1, principal], { from: admin });

      // Cash balance is growing
      const balance1 = await call(cash, 'balanceOf', [account1]);
      await sendRPC(web3, "evm_increaseTime", [30 * 60]);
      await sendRPC(web3, "evm_mine", []);
      const balance2 = await call(cash, 'balanceOf', [account1]);
      await sendRPC(web3, "evm_increaseTime", [2 * 60 * 60]);
      await sendRPC(web3, "evm_mine", []);
      const balance3 = await call(cash, 'balanceOf', [account1]);

      // Balance checks
      expect(balance2).greaterThan(balance1);
      expect(balance3).greaterThan(balance2);
    });
  });
});
