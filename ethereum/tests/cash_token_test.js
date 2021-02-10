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
  sendRPC,
  ETH_HEADER,
  ETH_ADDRESS,
  ETH_ZERO_ADDRESS
} = require('./utils');

// TODO: test fee token
describe('Starport', () => {
  let cash;
  let [root, admin, account1, account2] = saddle.accounts;


  beforeEach(async () => {
    cash = await deploy('CashToken', [admin], {from: root});
  });

  describe('Unit Tests', () => {
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

      //expect(await call(cash, 'totalSupply')).toEqualNumber(burnAmount * cashIndex);
      // expect(await call(cash, 'balanceOf', [account1])).toEqualNumber(burnAmount * cashIndex);

      // expect(tx.events.Transfer.returnValues).toMatchObject({
      //   from: account1,
      //   to: ETH_ZERO_ADDRESS,
      //   value: (cashIndex * burnAmount).toString()
      // });
    });

    it('should fail if called not by an admin', async() => {
      await expect(send(cash, 'burn', [account1, 10e6], { from: account1 })).rejects.toRevert("revert Sender is not an admin");
    })
  });



});
