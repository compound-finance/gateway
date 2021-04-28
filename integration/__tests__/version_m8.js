const { buildScenarios } = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 2000 } }
  ]
};

buildScenarios('Version m8', scen_info, [
  {
    only: true,
    name: "Test Running m8",
    info: {
      versions: ['m8'],
      genesis_version: 'm8',
      eth_opts: {
        version: 'm8',
      },
      validators: {
        alice: {
          version: 'm8',
        },
        bob: {
          version: 'm8',
        }
      },
    },
    scenario: async ({ ashley, usdc, chain, cash, starport }) => {
      await ashley.lock(1000, usdc);
      let notice = getNotice(await ashley.extract(100, cash));
      let signatures = await chain.getNoticeSignatures(notice);
      await starport.invoke(notice, signatures);
      // Don't check cash since we use a new RPC call that doesn't exist in m8

      await ashley.lock(100, usdc);
      expect(await ashley.tokenBalance(usdc)).toEqual(900);
      expect(await ashley.chainBalance(usdc)).toEqual(1100);
    }
  }
]);
