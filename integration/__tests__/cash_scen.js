const { buildScenarios } = require('../util/scenario');

let now = Date.now();

let cash_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } },
    { token: 'zrx', balances: { bert: 1000000 } }
  ],
  validators: ['alice', 'bob'],
  freeze_time: now,
  initial_yield: 300,
  initial_yield_start_ms: now
};

buildScenarios('Cash Scenarios', cash_scen_info, [
  {
    name: 'Cash Interest',
    scenario: async ({ ashley, bert, cash, chain, usdc, sleep }) => {
      await ashley.lock(1000, usdc);
      await ashley.transfer(10, cash, bert);
      await sleep(6000);
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toEqual(-10.01); // $10 + 1¢ transfer fee
      expect(await bert.chainBalance(cash)).toEqual(10);
      await chain.accelerateTime({years: 1});
      await sleep(6000); // Really this should just be `await chain.nextBlock()` or something
      expect(await ashley.chainBalance(usdc)).toEqual(1000);
      expect(await ashley.chainBalance(cash)).toBeCloseTo(-10.314849, 4); // $10.01 @ 3% for 1 Year Continously Compounding
      expect(await bert.chainBalance(cash)).toEqual(10.304545, 4); // // $10 @ 3% for 1 Year Continously Compounding
    }
  },
  {
    name: 'Collateral Borrowed Interest',
    scenario: async ({ ashley, bert, chuck, cash, chain, usdc, zrx, sleep }) => {
      await chain.setFixedRate(usdc, 500); // 5% APY fixed
      await bert.lock(1000000, zrx);
      await bert.transfer(1000, usdc, chuck);
      await sleep(6000);
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toEqual(-0.01); // 1¢ transfer fee
      expect(await chuck.chainBalance(cash)).toEqual(0);
      await chain.accelerateTime({years: 1});
      await sleep(6000); // Really this should just be `await chain.nextBlock()` or something
      expect(await bert.chainBalance(usdc)).toEqual(-1000);
      expect(await chuck.chainBalance(usdc)).toEqual(1000);
      expect(await bert.chainBalance(cash)).toBeCloseTo(-51.53272669767585, 3); // -50 * Math.exp(0.03) - 0.01
      expect(await chuck.chainBalance(cash)).toBeCloseTo(51.52272669767585, 3); // 50 * Math.exp(0.03)
    }
  }
]);
