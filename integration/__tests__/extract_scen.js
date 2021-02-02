const {
  years,
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let extract_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
};

async function lockUSDC({ ashley, zrx }) {
  await ashley.lock(100, zrx);
  expect(await ashley.tokenBalance(zrx)).toEqual(900);
  expect(await ashley.chainBalance(zrx)).toEqual(100);
}

buildScenarios('Extract Scenarios', extract_scen_info, { beforeEach: lockUSDC }, [
  {
    name: "Extract Collateral",
    scenario: async ({ ashley, zrx, chain, starport, log }) => {
      let notice = getNotice(await ashley.extract(50, zrx));
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await ashley.tokenBalance(zrx)).toEqual(900);
      await starport.unlock(notice, signatures);
      expect(await ashley.tokenBalance(zrx)).toEqual(950);
      expect(await ashley.chainBalance(zrx)).toEqual(50);
    }
  }
]);
