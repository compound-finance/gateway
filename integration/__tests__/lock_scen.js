const {
  years,
  buildScenarios
} = require('../util/scenario');

let lock_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } }
  ],
};

buildScenarios('Lock Scenarios', lock_scen_info, [
  {
    name: 'Lock Collateral',
    scenario: async ({ ashley, usdc, chain }) => {
      await ashley.lock(100, usdc, false);
      expect(await ashley.tokenBalance(usdc)).toEqual(900);
      await chain.waitForEthProcessEvent('cash', 'GoldieLocks'); // Replace with real event
      expect(await ashley.chainBalance(usdc)).toEqual(101);
    }
  },
  {
    name: 'Lock Eth',
    scenario: async ({ ashley, chain, ether }) => {
      await ashley.lock(0.01, ether, false);
      expect(await ashley.tokenBalance(ether)).toEqual(99.99);
      await chain.waitForEthProcessEvent('cash', 'GoldieLocks'); // Replace with real event
      expect(await ashley.chainBalance(ether)).toEqual(0.01);
    }
  },
  {
    skip: true,
    name: 'Lock Too Little Collateral',
    scenario: async ({ ashley, usdc, chain }) => {
      await ashley.lock(0.1, usdc, false);
      expect(await ashley.tokenBalance(usdc)).toEqual(999.9);
      let failure = await chain.waitForEthProcessFailure();
      expect(failure).toHaveReason('MinTxValueNotMet');
      expect(await ashley.chainBalance(usdc)).toEqual(0);
    }
  },
  {
    skip: true,
    name: 'Lock Collateral Events',
    scenario: async ({ ashley, usdc }) => {
      let tx = await ashley.lock(100, usdc);
      expect(tx).toHaveEthEvent('Lock', {
        asset: usdc.ethAddress(),
        holder: ashley.ethAddress(),
        amount: usdc.toWeiAmount(100)
      });
      expect(await ashley.tokenBalance(usdc)).toEqual(900);
      let event = await chain.waitForEvent('cash', 'GoldieLocks');
      expect(event).toChainEventEqual({
        'Yabba': 'Dabba'
      });
      expect(await ashley.chainBalance(usdc)).toEqual(100);
    }
  },
  // TODO: Lock Eth Events
  {
    skip: true,
    name: 'Lock Collateral - Insufficient Balance',
    scenario: async ({ ashley, usdc }) => {
      await expect(ashley.lock(2000, usdc)).toEthRevert('insufficient balance');
      expect(await ashley.tokenBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(usdc)).toEqual(0);
    }
  },
  {
    skip: true,
    name: 'Lock Collateral - Reverting Token',
    info: {
      tokens: [{ token: 'reverter', balances: { ashley: 1000 } }]
    },
    scenario: async ({ ashley, reverter }) => {
      await expect(ashley.supply(1000, reverter)).toEthRevert('token reversion');
      expect(await ashley.tokenBalance(reverter)).toEqual(1000);
      expect(await ashley.chainBalance(reverter)).toEqual(0);
    }
  },
  {
    skip: true,
    name: 'Supply Collateral - Fee Token',
    info: {
      tokens: [{ token: 'fee', balances: { ashley: 1000 } }]
    },
    scenario: async ({ ashley, fee }) => {
      await ashley.supply(100, fee);
      expect(await ashley.tokenBalance(fee)).toEqual(900);
      expect(await ashley.chainBalance(fee)).toEqual(80);
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
  // TODO: Test below minimum threshold
]);
