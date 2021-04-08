const {
  years,
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let transfer_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
};

async function lockUSDC({ ashley, bert, zrx }) {
  await ashley.lock(100, zrx);
  expect(await ashley.tokenBalance(zrx)).toEqual(900);
  expect(await ashley.chainBalance(zrx)).toEqual(100);
  expect(await bert.chainBalance(zrx)).toEqual(0);
}

buildScenarios('Transfer Scenarios', transfer_scen_info, { beforeEach: lockUSDC }, [
  {
    name: "Transfer Collateral",
    scenario: async ({ ashley, bert, zrx, chain, starport, log }) => {
      await ashley.transfer(50, zrx, bert);
      expect(await ashley.tokenBalance(zrx)).toEqual(900);
      expect(await ashley.chainBalance(zrx)).toEqual(50);
      expect(await bert.chainBalance(zrx)).toEqual(50);
    }
  },
  {
    name: "Transfer Cash",
    scenario: async ({ ashley, bert, zrx, chain, starport, cash }) => {
      await ashley.transfer(10, cash, bert);
      expect(await ashley.tokenBalance(cash)).toEqual(0);
      expect(await bert.tokenBalance(cash)).toEqual(0);
      expect(await ashley.cash()).toBeCloseTo(-10.01, 4);
      expect(await bert.cash()).toBeCloseTo(10, 4);
    }
  },
  {
    name: "Transfer Cash Max",
    scenario: async ({ ashley, bert, chuck, zrx, chain, starport, cash }) => {
      await ashley.transfer(10, cash, bert);
      await bert.transfer('Max', cash, chuck); // This is failing due to Insufficient Liquidity (!)
      let ashleyCash = await ashley.cash();
      let bertCash = await bert.cash();
      let chuckCash = await chuck.cash();

      // TODO: Fix checks below
      expect(ashleyCash).toBeCloseTo(-10.01, 4);
      expect(bertCash).toEqual(0, 4);
      expect(chuckCash).toEqual(10, 4);
    }
  }
]);
