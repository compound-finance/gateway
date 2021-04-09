const {
  years,
  buildScenarios
} = require('../util/scenario');
const { getEventData, getNotice } = require('../util/substrate');
const { bytes32 } = require('../util/util');

let lock_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } }
  ],
  validators: ['alice']
};

async function getCash({ ashley, usdc, cash, chain, starport }) {
  await ashley.lock(1000, usdc);
  let notice = getNotice(await ashley.extract(100, cash));
  let signatures = await chain.getNoticeSignatures(notice);
  await starport.invoke(notice, signatures);
  expect(await ashley.tokenBalance(cash)).toBeCloseTo(100);
  expect(await ashley.chainBalance(cash)).toBeCloseTo(-100);
}

buildScenarios('Lock Scenarios', lock_scen_info, [
  {
    name: 'Lock Collateral',
    scenario: async ({ ashley, usdc, chain }) => {
      await ashley.lock(100, usdc);
      expect(await ashley.tokenBalance(usdc)).toEqual(900);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
    }
  },
  {
    name: 'Lock Eth',
    scenario: async ({ ashley, chain, ether }) => {
      await ashley.lock(0.01, ether);
      expect(await ashley.tokenBalance(ether)).toEqual(99.99);
      expect(await ashley.chainBalance(ether)).toEqual(0.01);
      expect(await ashley.chainBalanceFromRPC(ether)).toEqual(0.01);
    }
  },
  {
    before: getCash,
    name: 'Lock Cash',
    scenario: async ({ ashley, cash, chain }) => {
      await ashley.lock(100, cash);
      expect(await ashley.tokenBalance(cash)).toBeCloseTo(0);
      expect(await ashley.chainBalance(cash)).toBeCloseTo(0);
    }
  },
  {
    name: 'No minimum on lock collateral',
    scenario: async ({ ashley, usdc, chain }) => {
      await ashley.lock(0.1, usdc);
      expect(await ashley.tokenBalance(usdc)).toEqual(999.9);
      expect(await ashley.chainBalance(usdc)).toEqual(0.1);
    }
  },
  {
    name: 'Lock Collateral Events',
    scenario: async ({ ashley, usdc }) => {
      let {tx, event} = await ashley.lock(100, usdc);
      expect(tx).toHaveEthEvent('Lock', {
        asset: usdc.ethAddress(),
        sender: ashley.ethAddress(),
        chain: 'ETH',
        recipient: bytes32(ashley.ethAddress()),
        amount: usdc.toWeiAmount(100).toString()
      });
      expect(event).toMatchChainEvent({
        AssetAmount: 100000000,
        ChainAccount: { Eth: ashley.ethAddress().toLowerCase() },
        ChainAsset: { Eth: usdc.ethAddress().toLowerCase() }
      });
    }
  },
  {
    name: 'Lock Eth Events',
    scenario: async ({ ashley, ether }) => {
      let {tx, event} = await ashley.lock(0.01, ether);
      expect(tx).toHaveEthEvent('Lock', {
        asset: ether.ethAddress(),
        sender: ashley.ethAddress(),
        chain: 'ETH',
        recipient: bytes32(ashley.ethAddress()),
        amount: ether.toWeiAmount(0.01).toString()
      });
      expect(event).toMatchChainEvent({
        AssetAmount: "0x0000000000000000002386f26fc10000",
        ChainAccount: { Eth: ashley.ethAddress().toLowerCase() },
        ChainAsset: { Eth: ether.ethAddress().toLowerCase() }
      });
    }
  },
  {
    before: getCash,
    name: 'Lock Cash Events',
    scenario: async ({ ashley, cash, chain }) => {
      let {tx, event} = await ashley.lock(100, cash);
      expect(tx).toHaveEthEvent('LockCash', {
        sender: ashley.ethAddress(),
        chain: 'ETH',
        recipient: bytes32(ashley.ethAddress()),
        amount: cash.toWeiAmount(100).toString()
      });
      // Note: we don't differentiate between the two "ChainAccount" arguments here.
      expect(event).toMatchChainEvent({
        ChainAccount: { Eth: ashley.ethAddress().toLowerCase() },
      });
      let data = getEventData(event);
      expect(data.CashPrincipalAmount).toBeCloseTo(99999996, -1); // TODO: Check this better
      expect(data.CashIndex).toBeWithinRange(1000000000000000000, 1000000100000000000);
    }
  },
  {
    name: 'Not Lock Collateral with Insufficient Balance',
    scenario: async ({ ashley, usdc }) => {
      await expect(ashley.lock(2000, usdc)).rejects.toEthRevert('revert');
      expect(await ashley.tokenBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(usdc)).toEqual(0);
    }
  },
  {
    name: 'Not Lock Cash with Insufficient Balance',
    scenario: async ({ ashley, cash }) => {
      await expect(ashley.lock(2000, cash)).rejects.toEthRevert('revert');
      expect(await ashley.tokenBalance(cash)).toEqual(0);
      expect(await ashley.chainBalance(cash)).toEqual(0);
    }
  },
  {
    name: 'Supply Fee Token',
    info: {
      tokens: [{ token: 'fee', balances: { ashley: 1000 } }]
    },
    scenario: async ({ ashley, fee }) => {
      expect(await ashley.tokenBalance(fee)).toEqual(500);
      await ashley.lock(200, fee);
      expect(await ashley.tokenBalance(fee)).toEqual(300);
      expect(await ashley.chainBalance(fee)).toEqual(100);
    }
  },
  {
    skip: true,
    name: 'Supply Collateral With Interest',
    scenario: async ({ ashley, cash, chain, usdc }) => {
      await chain.freezeTime(chain.timestamp());
      await chain.setFixedRateBPS(usdc, 100n);
      await chain.setPriceCents(usdc, 100n);
      await ashley.supply(1000, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toEqual(0);
      await chain.accelerateTime(years(1));
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toEqual(10);
    }
  }
]);
