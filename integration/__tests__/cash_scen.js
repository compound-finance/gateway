const { buildScenarios } = require('../util/scenario');

let now = Date.now();

let cash_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } },
    { token: 'zrx', balances: { bert: 1000000 } },
    { token: 'comp' }
  ],
  validators: ['alice', 'bob'],
  freeze_time: now,
  initial_yield: 300,
  initial_yield_start_ms: now
};

buildScenarios('Cash Scenarios', cash_scen_info, [
  {
    name: 'Cash Interest',
    scenario: async ({ ashley, bert, cash, chain, usdc }) => {
      await ashley.lock(1000, usdc);
      await ashley.transfer(10, cash, bert);
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toEqual(-10.01); // $10 + 1¢ transfer fee
      expect(await bert.chainBalance(cash)).toEqual(10);
      await chain.accelerateTime({years: 1});
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toBeCloseTo(-10.314849, 4); // $10.01 @ 3% for 1 Year Continously Compounding
      expect(await bert.chainBalance(cash)).toBeCloseTo(10.304545, 4); // // $10 @ 3% for 1 Year Continously Compounding
    }
  },
  {
    name: 'Collateral Borrowed Interest Lump Sum',
    scenario: async ({ bert, chuck, cash, chain, usdc, zrx }) => {
      await chain.setFixedRate(usdc, 500); // 5% APY fixed
      await bert.lock(1000000, zrx);
      await bert.transfer(1000, usdc, chuck);
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toEqual(-0.01); // 1¢ transfer fee
      expect(await chuck.chainBalance(cash)).toEqual(0);
      await chain.accelerateTime({years: 1});
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toBeCloseTo(-51.53272669767585, 3); // -50 * Math.exp(0.03) - 0.01
      expect(await chuck.chainBalance(cash)).toBeCloseTo(51.52272669767585, 3); // 50 * Math.exp(0.03)
    }
  },
  {
    name: 'Collateral Borrowed Interest 12-Month Chunked',
    scenario: async ({ ashley, bert, chuck, cash, chain, usdc, zrx }) => {
      await chain.setFixedRate(usdc, 500); // 5% APY fixed
      await bert.lock(1000000, zrx);
      await bert.transfer(1000, usdc, chuck);
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toEqual(-0.01); // 1¢ transfer fee
      expect(await chuck.chainBalance(cash)).toEqual(0);
      for (const i in [...new Array(12)]) {
        await chain.accelerateTime({months: 1});
      }
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toBeCloseTo(-50.79, 1); // ~ -50*(1+0.015) - 0.01
      expect(await chuck.chainBalance(cash)).toBeCloseTo(50.78, 1); // ~ -50*(1+0.015)
    }
  },
  {
    name: 'Multi-Collateral and Cash Netting',
    scenario: async ({ ashley, bert, chuck, cash, chain, comp, usdc, zrx }) => {
      await chain.setFixedRate(usdc, 500); // 5% APY fixed
      await chain.setFixedRate(comp, 1000); // 10% APY fixed
      await bert.lock(1000000, zrx);
      await bert.transfer(300.01, cash, chuck);
      await bert.transfer(1000, usdc, chuck);
      await chuck.transfer(1, comp, bert);
      // Chuck has +1000 USDC @ 5% [Price=$1] [Util=100%]
      // Chuck has -1 COMP @ 10% [Price=$229.125] [Util=100%]
      // Chuck has 300 Cash @ 3% APY
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await chuck.chainBalance(comp)).toEqual(-1);
      expect(await chuck.chainBalance(cash)).toEqual(300);
      await chain.accelerateTime({years: 1});
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await chuck.chainBalance(comp)).toEqual(-1);
      /*
          { Cash }  {  USDC Interest  }   {  Comp Interest  }   {  Cash APY   }

        (   300   +   1000 * 1 * 0.05   -  1 * 229.125 * 0.1 ) * Math.exp(0.03)
      */
      expect(await chuck.chainBalance(cash)).toBeCloseTo(337.048797374521, 3);
    }
  }
]);
