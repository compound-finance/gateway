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
    skip: true,
    name: "Transfer Cash",
    scenario: async ({ ashley, bert, zrx, chain, starport, cash }) => {
      let notice = await ashley.extract(50, cash);
      expect(await cash.getCashPrincipal(ashley)).toEqual(5000); // ??
      expect(await ashley.tokenBalance(cash)).toEqual(50);
      expect(await ashley.chainBalance(cash)).toEqual(-50);
      expect(await bert.chainBalance(cash)).toEqual(50);
    }
  }
]);
