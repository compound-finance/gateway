const {
  years,
  buildScenarios
} = require('../util/scenario');

let extract_scen_info = {
  tokens: [
    { token: "usdc", balances: { ashley: 1000 } }
  ],
};

async function lockUSDC({ ashley, usdc }) {
  await ashley.lock(100, usdc, true);
  expect(await ashley.tokenBalance(usdc)).toEqual(900);
  expect(await ashley.chainBalance(usdc)).toEqual(100);
}

buildScenarios('Extract Scenarios', extract_scen_info, { beforeEach: lockUSDC }, [
  {
    skip: true,
    name: "Extract Collateral",
    scenario: async ({ ashley, usdc, chain, starport }) => {
      let extract = await ashley.extract(50, usdc);
      let { notice, signatures } = chain.getNotice(extract);
      await starport.unlock(notice, signatures);
      expect(await ashley.tokenBalance(usdc)).toEqual(950);
      expect(await ashley.chainBalance(usdc)).toEqual(50);
    }
  }
]);
