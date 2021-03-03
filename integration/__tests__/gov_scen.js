const {
  buildScenarios
} = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');

let gov_scen_info = {
  tokens: [
    { token: "zrx" }
  ],
};

buildScenarios('Gov Scenarios', gov_scen_info, [
  {
    name: "Update Interest Rate Model by Governance",
    scenario: async ({ ctx, zrx, chain, starport, sleep }) => {
      let newKink = {
        Kink: {
          zero_rate: 100,
          kink_rate: 500,
          kink_utilization: 80,
          full_rate: 1000
        }
      };
      let extrinsic = ctx.api().tx.cash.setRateModel(zrx.toChainAsset(), newKink);
      await starport.executeProposal("Update ZRX Interest Rate Model", [extrinsic]);
      expect(await chain.interestRateModel(zrx)).toEqual(newKink);
    }
  },
  {
    name: "Upgrade Chain WASM [Set Code]",
    info: {
      versions: ['v1.1.1'],
      genesis_version: 'v1.1.1',
      validators: {
        alice: {
          version: 'v1.1.1',
        }
      }
    },
    scenario: async ({ ctx, zrx, chain, starport, curr, sleep }) => {
      expect(await chain.getSemVer()).toEqual([1, 1, 1]);

      let event = await chain.setCode(await curr.wasm());

      expect(await chain.getSemVer()).toEqual([1, 2, 2]);
    }
  },
  {
    skip: true,
    name: "Upgrade Chain WASM [Allow Next Code]",
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
  },
  {
    name: "Update Auth",
    scenario: async ({ ctx, chain, starport, validators }) => {
      const alice = validators.validatorInfoMap.alice;
      const alice_account_id = alice.aura_key;
      const newAuthsRaw = [[ctx.actors.keyring.decodeAddress(alice_account_id), { eth_address: alice.eth_account }]];
      let extrinsic = ctx.api().tx.cash.changeAuthorities(newAuthsRaw);
      await starport.executeProposal("Update authorities", [extrinsic]);
      const pending = await chain.pendingCashValidators();

      const newAuths = [[alice_account_id, { eth_address: alice.eth_account }]];
      expect(pending).toEqual(newAuths);

      await chain.waitUntilSession(3);
      const newVals = await chain.cashValidators();
      expect(newVals).toEqual(newAuths);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths).toEqual([alice_account_id]);
      
      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths).toEqual([alice.grandpa_key]);

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths).toEqual([alice.aura_key]);
    }
  },
  {
    name: "Read Extrinsic from Event",
    scenario: async ({ ctx, zrx, chain, starport }) => {
      let newKink = {
        Kink: {
          zero_rate: 100,
          kink_rate: 500,
          kink_utilization: 80,
          full_rate: 1000
        }
      };
      let extrinsic = ctx.api().tx.cash.setRateModel(zrx.toChainAsset(), newKink);
      let { event } = await starport.executeProposal("Update ZRX Interest Rate Model", [extrinsic]);
      let [[[data]]] = event.data;

      expect(decodeCall(ctx.api(), data)).toEqual({
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
