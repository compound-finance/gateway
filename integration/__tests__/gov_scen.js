const { buildScenarios } = require('../util/scenario');
const { decodeCall } = require('../util/substrate');

let gov_scen_info = {
  tokens: [
    { token: "zrx" }
  ]
};

buildScenarios('Gov Scenarios', gov_scen_info, [
  {
    name: "Update Interest Rate Model by Governance",
    scenario: async ({ api, zrx, chain, starport }) => {
      let newKink = {
        kink: {
          zero_rate: 100,
          kink_rate: 500,
          kink_utilization: 80,
          full_rate: 1000
        }
      };
      let extrinsic = api.tx.cash.setRateModel(zrx.toChainAsset(), newKink);
      await starport.executeProposal("Update ZRX Interest Rate Model", [extrinsic]);
      expect(await chain.interestRateModel(zrx)).toEqual(newKink);
    }
  },
  {
    skip: true, // TODO FIX SCEN
    name: "Upgrade Chain WASM [Allow Next Code]",
    info: {
      versions: ['m7'],
      genesis_version: 'm7',
      validators: {
        alice: {
          version: 'm7',
          extra_versions: ['curr']
        }
      },
    },
    scenario: async ({ api, zrx, chain, starport, curr }) => {
      expect(await chain.getSemVer()).toEqual([1, 7, 1]);
      let currHash = await curr.hash();
      let extrinsic = api.tx.cash.allowNextCodeWithHash(currHash);

      await starport.executeProposal("Upgrade from m7 to Current [Allow Next Code]", [extrinsic]);

      expect(await chain.nextCodeHash()).toEqual(currHash);

      let event = await chain.setNextCode(await curr.wasm(), curr, false);
      expect(event).toEqual({
        CodeHash: currHash,
        DispatchResult: {
          Ok: []
        }
      });

      expect(await chain.getSemVer()).not.toEqual([1, 7, 1]);
    }
  },
  {
    name: "Read Extrinsic from Event",
    scenario: async ({ api, zrx, chain, starport }) => {
      let newKink = {
        Kink: {
          zero_rate: 100,
          kink_rate: 500,
          kink_utilization: 80,
          full_rate: 1000
        }
      };
      let extrinsic = api.tx.cash.setRateModel(zrx.toChainAsset(), newKink);
      let { event } = await starport.executeProposal("Update ZRX Interest Rate Model", [extrinsic]);
      let [[[data]]] = event.data;

      expect(decodeCall(api, data)).toEqual({
        section: "cash",
        method: "setRateModel",
        args: [
          zrx.toChainAsset(true),
          {
            "Kink": {
              "full_rate": "1,000",
              "kink_rate": "500",
              "kink_utilization": "80",
              "zero_rate": "100"
            }
          }
        ]
      });
    }
  }
]);
