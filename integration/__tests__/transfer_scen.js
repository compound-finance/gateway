const { buildScenarios } = require('../util/scenario');
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
      await ashley.transfer(2, cash, bert);
      await bert.transfer('Max', cash, chuck);
      expect(await bert.cash()).toBeCloseTo(0, 4);
      expect(await chuck.cash()).toBeCloseTo(1.99, 4);
    }
  },
  {
    name: "Transfer Cash Max Insufficient",
    scenario: async ({ ashley, bert, chuck, zrx, chain, starport, cash }) => {
      await ashley.transfer(2, cash, bert);
      await bert.transfer(1.985, cash, ashley);
      expect(await bert.cash()).toBeCloseTo(0.005, 4);
      await expect(bert.transfer('Max', cash, chuck)).rejects.toThrow(/insufficientCashForMaxTransfer/);
      expect(await bert.cash()).toBeCloseTo(0.005, 4);
      expect(await chuck.cash()).toBeCloseTo(0, 4);
    }
  }
]);
