const {
  sendRPC,
  ETH_ZERO_ADDRESS
} = require('./utils');

describe('CashToken', () => {
  let cash;
  let [root, admin, account1, account2, account3] = saddle.accounts;


  beforeEach(async () => {
    cash = await deploy('CashToken', [admin], {from: root});
  });

  describe('#constructor', () => {
    it('should have correct references', async () => {
      expect(await call(cash, 'admin')).toMatchAddress(admin);
      cashYieldAndIndex = await call(cash, 'cashYieldAndIndex');
      expect(cashYieldAndIndex.index).toEqualNumber(1e6);
      expect(cashYieldAndIndex.yield).toEqualNumber(0);
    });
  });

  describe('#setFutureYield', () => {
    it('should set correct current and next indexes, yields and startTimes', async () => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp + 30 * 60;

      const yieldAndIndex_before = await call(cash, 'cashYieldAndIndex');
      const startAt_before = await call(cash, 'cashYieldStartAt');

      // Update future yield, first change
      await send(cash, 'setFutureYield', [43628, 1e6, nextYieldTimestamp], { from: admin });
      const yieldAndIndex_change = await call(cash, 'cashYieldAndIndex');
      const startAt_change = await call(cash, 'cashYieldStartAt');
      const nextYieldAndIndex_change = await call(cash, 'nextCashYieldAndIndex');
      const nextStartAt_change = await call(cash, 'nextCashYieldStartAt');

      expect(yieldAndIndex_change.yield).toEqualNumber(yieldAndIndex_before.yield);
      expect(yieldAndIndex_change.index).toEqualNumber(yieldAndIndex_before.index);
      expect(startAt_change).toEqualNumber(startAt_before);
      expect(nextYieldAndIndex_change.yield).toEqualNumber(43628);
      expect(nextYieldAndIndex_change.index).toEqualNumber(1e6);
      expect(nextStartAt_change).toEqualNumber(nextYieldTimestamp);

      await sendRPC(web3, "evm_increaseTime", [31 * 60]);

      // Update future yield, second change, current yield, index and time are set to previous next values
      await send(cash, 'setFutureYield', [43629, 11e5, nextYieldTimestamp + 60 * 60], { from: admin });
      const yieldAndIndex_change2 = await call(cash, 'cashYieldAndIndex');
      const startAt_change2 = await call(cash, 'cashYieldStartAt');
      const nextYieldAndIndex_change2 = await call(cash, 'nextCashYieldAndIndex');
      const nextStartAt_change2 = await call(cash, 'nextCashYieldStartAt');

      expect(yieldAndIndex_change2.yield).toEqualNumber(nextYieldAndIndex_change.yield);
      expect(yieldAndIndex_change2.index).toEqualNumber(nextYieldAndIndex_change.index);
      expect(startAt_change2).toEqualNumber(nextStartAt_change);
      expect(nextYieldAndIndex_change2.yield).toEqualNumber(43629);
      expect(nextYieldAndIndex_change2.index).toEqualNumber(11e5);
      expect(nextStartAt_change2).toEqualNumber(nextYieldTimestamp + 60 * 60);
    });

    it('should fail if called not by an admin', async() => {
      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);
      const nextYieldTimestamp = block.timestamp + 30 * 60;
      await expect(send(cash, 'setFutureYield', [43628, 1e6, nextYieldTimestamp], { from: account1 })).rejects.toRevert("revert Sender is not an admin");
    })
  });

  describe('#mint', () => {
    it('should mint tokens and emit `Transfer` event', async () => {
      expect(await call(cash, 'totalSupply')).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      const tx = await send(cash, 'mint', [account1, amountPrincipal], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber(amountPrincipal * cashIndex);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(amountPrincipal * cashIndex);

      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: ETH_ZERO_ADDRESS,
        to: account1,
        value: (cashIndex * amountPrincipal).toString()
      });
    });

    it('should fail if called not by an admin', async() => {
      await expect(send(cash, 'mint', [account1, 10e6], { from: account1 })).rejects.toRevert("revert Sender is not an admin");
    })
  });

  describe('#burn', () => {
    it('should burn tokens and emit `Transfer` event', async () => {
      // Let's mint tokens first, to have something to burn
      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      const burnAmount = 5e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });

      // An attempt to burn tokens
      const tx = await send(cash, 'burn', [account1, burnAmount], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber(burnAmount * cashIndex);
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(burnAmount * cashIndex);

      expect(tx.events.Transfer.returnValues).toMatchObject({
        from: account1,
        to: ETH_ZERO_ADDRESS,
        value: (cashIndex * burnAmount).toString()
      });
    });

    it('should fail if called not by an admin', async() => {
      await expect(send(cash, 'burn', [account1, 10e6], { from: account1 })).rejects.toRevert("revert Sender is not an admin");
    })
  });

  describe('#totalSupply', () => {
    it('should return total supply of cash', async () => {
      expect(await call(cash, 'totalSupply')).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal1 = 10e6;
      const amountPrincipal2 = 5e6;
      await send(cash, 'mint', [account1, amountPrincipal1], { from: admin });
      await send(cash, 'mint', [account2, amountPrincipal2], { from: admin });

      expect(await call(cash, 'totalSupply')).toEqualNumber((amountPrincipal1 + amountPrincipal2) * cashIndex);
    });
  });

  describe('#balanceOf', () => {
    it('should return balance of Cash tokens for given account', async () => {
      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(0);
      expect(await call(cash, 'balanceOf', [account2])).toEqualNumber(0);

      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal1 = 10e6;
      const amountPrincipal2 = 5e6;
      await send(cash, 'mint', [account1, amountPrincipal1], { from: admin });
      await send(cash, 'mint', [account2, amountPrincipal2], { from: admin });

      expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(amountPrincipal1 * cashIndex);
      expect(await call(cash, 'balanceOf', [account2])).toEqualNumber(amountPrincipal2 * cashIndex);
    });
  });

  describe('#name', () => {
    it('should return Cash token name', async () => {
      expect(await call(cash, 'name', [])).toEqual("SECRET, change");
    });
  });

  describe('#symbol', () => {
    it('should return Cash token symbol', async () => {
      expect(await call(cash, 'symbol', [])).toEqual("SECRET");
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
      const amountPrincipal = 10e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });

      const amount = amountPrincipal * cashIndex;
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

    it('should fail if recipient is wrong', async() => {
      await expect(send(cash, 'transfer', [account1, 1e6], { from: account1 })).rejects.toRevert("revert Invalid recipient");
    });

    it('should fail if not enough Cash tokens to transfer', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });

      const amount = amountPrincipal * cashIndex;
      // An attempt to transfer double amount
      await expect(send(cash, 'transfer', [account2, 2 * amount], { from: account1 })).rejects.toRevert("revert");
    });
  });

  describe('#transferFrom', () => {
    it('should transfer Cash tokens between users', async() => {
      // Mint tokes first to have something to transfer
      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });
      const amount = amountPrincipal * cashIndex;

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

    it('should fail if recipient is wrong', async() => {
      await expect(send(cash, 'transferFrom', [account1, account1, 1e6], { from: account1 })).rejects.toRevert("revert Invalid recipient");
    });

    it('should fail if not enough allowance', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });
      const amount = amountPrincipal * cashIndex;

      // Approve an account2 to move tokens on behalf of account1
      await send(cash, 'approve', [account2, amount / 2], {from: account1});

      // An attempt to transfer double the approved amount
      await expect(send(cash, 'transferFrom', [account1, account3, amount], { from: account2 })).rejects.toRevert("revert");
    });

    it('should fail if not enough Cash tokens to transfer', async() => {
      const cashIndex = await call(cash, 'getCashIndex');
      const amountPrincipal = 10e6;
      await send(cash, 'mint', [account1, amountPrincipal], { from: admin });
      const amount = amountPrincipal * cashIndex;

      // Approve an account2 to move tokens on behalf of account1
      await send(cash, 'approve', [account2, 2 * amount], {from: account1});

      // An attempt to transfer double the available amount
      await expect(send(cash, 'transferFrom', [account1, account3, 2 * amount], { from: account2 })).rejects.toRevert("revert");
    });
  });

  describe.only('#getCashIndex', () => {
    it.only('should return cashIndex', async() => {
      //const cashIndex1 = await call(cash, 'getCashIndex');
      //console.log('cash index 1 = ', cashIndex1);

      const blockNumber = await web3.eth.getBlockNumber();
      const block = await web3.eth.getBlock(blockNumber);

      const startAt = await call(cash, 'cashYieldStartAt', []);
      const yieldAndIndex = await call(cash, 'cashYieldAndIndex', []);
      console.log("startAt = ", startAt);
      console.log("yield and index = ", yieldAndIndex);
      const nextYieldTimestamp = block.timestamp + 30 * 60;
      await send(cash, 'setFutureYield', [43628, 1e6, nextYieldTimestamp], {from: admin});

      console.log("startAt = ", startAt);
      console.log("yield and index = ", yieldAndIndex);

      await sendRPC(web3, "evm_increaseTime", [365 * 24 * 60 * 60]);
      await sendRPC(web3, "evm_mine", []);

      const cashIndex2 = await call(cash, 'getCashIndex');
      console.log('cash index 2 = ', cashIndex2);

      // const blockNumber2 = await web3.eth.getBlockNumber();
      // const block2 = await web3.eth.getBlock(blockNumber2);
      // console.log("block timestamp = ", block2.timestamp);

      // console.log('cash index 2 = ', cashIndex2);
      // await sendRPC(web3, "evm_increaseTime", [24 * 60 * 60]);
      // await sendRPC(web3, "evm_mine", []);

      // const blockNumber3 = await web3.eth.getBlockNumber();
      // const block3 = await web3.eth.getBlock(blockNumber3);
      // console.log("block timestamp = ", block3.timestamp);

      // const cashIndex3 = await call(cash, 'getCashIndex');
      // console.log('cash index 3 = ', cashIndex3);
    });
  });

  // it.todo('#getCashIndex tests');
});
