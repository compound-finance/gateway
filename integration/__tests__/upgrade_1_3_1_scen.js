const {
  buildScenarios
} = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: "zrx" }
  ],
};

buildScenarios('Upgrade to 1.3.1', scen_info, [
  {
    name: "Upgrade from 1.2.1 to 1.3.1 with Live Events",
    info: {
      versions: ['v1.2.1'],
      genesis_version: 'v1.2.1',
      validators: {
        alice: {
          version: 'v1.2.1',
        }
      },
    },
    scenario: async ({ ctx, zrx, chain, starport, curr, sleep }) => {
      expect(await chain.getSemVer()).toEqual([1, 2, 1]);
      let currHash = await curr.hash();
      let extrinsic = ctx.api().tx.cash.allowNextCodeWithHash(currHash);

      await starport.executeProposal("Upgrade from v1.2.1 to Current [Allow Next Code]", [extrinsic]);

      expect(await chain.nextCodeHash()).toEqual(currHash);

      let event = await chain.setNextCode(await curr.wasm());
      expect(event).toEqual({
        CodeHash: currHash,
        DispatchResult: {
          Ok: []
        }
      });

      expect(await chain.getSemVer()).not.toEqual([1, 2, 1]);
    }
  }
]);
