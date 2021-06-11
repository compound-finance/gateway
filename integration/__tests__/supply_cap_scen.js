const { buildScenarios } = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let supply_cap_scen_info = {
  tokens: [
    { token: 'zrx', balances: { ashley: 1000 } }
  ],
  validators: ['alice', 'bob']
};

buildScenarios('Supply Cap Scenarios', supply_cap_scen_info, [
  {
    name: 'Set a new supply cap',
    scenario: async ({ api, ashley, chain, starport, zrx }) => {
      expect(await starport.supplyCap(zrx)).toEqual("1000000000000000000000000");
      let extrinsic = api.tx.cash.setSupplyCap(zrx.toChainAsset(), 1000);
      let { notice } = await starport.executeProposal("Set ZRX Supply Cap", [extrinsic], { awaitNotice: true });
      let signatures = await chain.getNoticeSignatures(notice, { signatures: 1 });
      await starport.invoke(notice, signatures);
      expect(await starport.supplyCap(zrx)).toEqual("1000");
    }
  }
  // TODO: Test the effects of the supply caps
]);
