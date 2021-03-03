const {
  buildScenarios
} = require('../util/scenario');
<<<<<<< HEAD:integration/__tests__/gov_scen.js
const { decodeCall, getEventData } = require('../util/substrate');

=======
const { decodeCall } = require('../util/substrate');
>>>>>>> set_keys invalid sig:integration/__tests__/gov_test.js
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
<<<<<<< HEAD:integration/__tests__/gov_scen.js
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
=======
    name: "Remove Auth",
>>>>>>> set_keys invalid sig:integration/__tests__/gov_test.js
    scenario: async ({ ctx, chain, starport, validators }) => {
      const alice = validators.validatorInfoMap.alice;
      const alice_account_id = alice.aura_key;
      const newAuthsRaw = [{ substrate_id: ctx.actors.keyring.decodeAddress(alice_account_id), eth_address: alice.eth_account }];
      let extrinsic = ctx.api().tx.cash.changeValidators(newAuthsRaw);
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
    name: "Add new auth with session keys",
    scenario: async ({ ctx, chain, starport, validators }) => {
      // spins up new validator charlie and adds to auth set
      const keyring = ctx.actors.keyring;
      const peer_id = "12D3KooW9qtwBHeQryg9mXBVMkz4YivUsj62g1tYBACUukKToKof";
      const node_key = "0x0000000000000000000000000000000000000000000000000000000000000002";
      const eth_private_key = "0xb1b07e7078273a09c64ef9bd52f49636535ba26624c7c75a57e1286b13c8f7ea";
      const eth_account = "0x9c00B0af5586aE099649137ca6d00a641aD30736";

      const newValidator = await validators.addValidator("Charlie", { peer_id, node_key, eth_private_key, eth_account });

      const newValidatorKeys = await chain.rotateKeys(newValidator);

      const charlie = keyring.createFromUri("//Charlie");
      const charlieCompoundId = charlie.address;
      await chain.setKeys(charlie, newValidatorKeys);

      const { alice, bob } = validators.validatorInfoMap;
      const toValKeys = (substrateId, ethAccount) => {return  {substrate_id: keyring.decodeAddress(substrateId), eth_address: ethAccount} };
      const allAuthsRaw = [
        toValKeys(alice.aura_key, alice.eth_account),
        toValKeys(bob.aura_key, bob.eth_account),
        toValKeys(charlieCompoundId, eth_account),
      ];

      const extrinsic = ctx.api().tx.cash.changeValidators(allAuthsRaw);
      await starport.executeProposal("Update authorities", [extrinsic]);

      await chain.waitUntilSession(3);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths.sort()).toEqual([alice.aura_key, bob.aura_key, charlieCompoundId].sort());

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths.sort()).toEqual([alice.aura_key, bob.aura_key, keyring.encodeAddress(newValidatorKeys.aura)].sort());

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths.sort()).toEqual([alice.grandpa_key, bob.grandpa_key, keyring.encodeAddress(newValidatorKeys.grandpa)].sort());
    }
  },
  {
    name: "Does not add auth w/o session keys",
    scenario: async ({ ctx, chain, starport, validators }) => {
      // spins up new validator charlie, doesnt add session keys, change validators should fail
      const keyring = ctx.actors.keyring;
      const peer_id = "12D3KooW9qtwBHeQryg9mXBVMkz4YivUsj62g1tYBACUukKToKof";
      const node_key = "0x0000000000000000000000000000000000000000000000000000000000000002";
      const eth_private_key = "0xb1b07e7078273a09c64ef9bd52f49636535ba26624c7c75a57e1286b13c8f7ea";
      const eth_account = "0x9c00B0af5586aE099649137ca6d00a641aD30736";

      await validators.addValidator("Charlie", { peer_id, node_key, eth_private_key, eth_account });

      const charlie = keyring.createFromUri("//Charlie");
      const charlieCompoundId = charlie.address;

      const { alice, bob } = validators.validatorInfoMap;
      const toValKeys = (substrateId, ethAccount) => {return  {substrate_id: keyring.decodeAddress(substrateId), eth_address: ethAccount} };
      const allAuthsRaw = [
        toValKeys(alice.aura_key, alice.eth_account),
        toValKeys(bob.aura_key, bob.eth_account),
        toValKeys(charlieCompoundId, eth_account),
      ];

      const extrinsic = ctx.api().tx.cash.changeValidators(allAuthsRaw);
      await starport.executeProposal("Update authorities", [extrinsic]);

      await chain.waitUntilSession(3);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths.sort()).toEqual([alice.aura_key, bob.aura_key].sort());

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths.sort()).toEqual([alice.aura_key, bob.aura_key].sort());

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths.sort()).toEqual([alice.grandpa_key, bob.grandpa_key].sort());
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
