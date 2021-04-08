const {
  buildScenarios
} = require('../util/scenario');
const { decodeCall } = require('../util/substrate');
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
    skip: true,
    name: "Upgrade Chain WASM [Allow Next Code]",
    info: {
      versions: ['m2'],
      genesis_version: 'm2',
      validators: {
        alice: {
          version: 'm2',
        }
      },
    },
    scenario: async ({ ctx, zrx, chain, starport, curr, sleep }) => {
      expect(await chain.getSemVer()).toEqual([1, 2, 1]);
      let currHash = await curr.hash();
      let extrinsic = ctx.api().tx.cash.allowNextCodeWithHash(currHash);

      await starport.executeProposal("Upgrade from m2 to Current [Allow Next Code]", [extrinsic]);

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
    name: "Remove Auth",
    scenario: async ({ ctx, chain, starport, validators }) => {
      const alice = validators.validatorInfoMap.alice;
      const newAuthsRaw = [{ substrate_id: ctx.actors.keyring.decodeAddress(alice.aura_key), eth_address: alice.eth_account }];

      let extrinsic = ctx.api().tx.cash.changeValidators(newAuthsRaw);

      await starport.executeProposal("Update authorities", [extrinsic]);

      const pending = await chain.pendingCashValidators();

      const newAuths = [[alice.aura_key, { eth_address: alice.eth_account }]];
      expect(pending).toEqual(newAuths);

      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);
      const newVals = await chain.cashValidators();
      expect(newVals).toEqual(newAuths);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths).toEqual([alice.aura_key]);

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths).toEqual([alice.grandpa_key]);

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths).toEqual([alice.aura_key]);
    }
  },
  {
    name: "Add new auth with session keys",
    scenario: async ({ ctx, chain, starport, validators }) => {
      // Spin up new validator Charlie and add to auth set
      const keyring = ctx.actors.keyring;
      const eth_private_key = "0xb1b07e7078273a09c64ef9bd52f49636535ba26624c7c75a57e1286b13c8f7ea";
      const eth_account = "0x9c00B0af5586aE099649137ca6d00a641aD30736";
      const node_key = "0x0000000000000000000000000000000000000000000000000000000000000003";
      const peer_id = "12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x";
      const spawn_args = ['--charlie'];

      const newValidator = await validators.addValidator("Charlie", { peer_id, node_key, eth_private_key, eth_account, spawn_args });

      const newValidatorKeys = await chain.rotateKeys(newValidator);

      const charlie = keyring.createFromUri("//Charlie");
      const charlieGatewayId = charlie.address;
      await chain.setKeys(charlie, newValidatorKeys);

      const { alice, bob } = validators.validatorInfoMap;
      const toValKeys = (substrateId, ethAccount) => {return  {substrate_id: keyring.decodeAddress(substrateId), eth_address: ethAccount} };
      const allAuthsRaw = [
        toValKeys(alice.aura_key, alice.eth_account),
        toValKeys(bob.aura_key, bob.eth_account),
        toValKeys(charlieGatewayId, eth_account),
      ];

      const extrinsic = ctx.api().tx.cash.changeValidators(allAuthsRaw);
      const {notice} = await starport.executeProposal("Update authorities", [extrinsic], {awaitNotice: true});

      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths.sort()).toEqual([alice.aura_key, bob.aura_key, charlieGatewayId].sort());

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths.sort()).toEqual([alice.aura_key, bob.aura_key, keyring.encodeAddress(newValidatorKeys.aura)].sort());

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths.sort()).toEqual([alice.grandpa_key, bob.grandpa_key, keyring.encodeAddress(newValidatorKeys.grandpa)].sort());


      let signatures = await chain.getNoticeSignatures(notice);
      let tx = await starport.invoke(notice, signatures);

      const starportAuths = await starport.getAuthorities();
      expect(starportAuths.slice().sort()).toEqual([alice.eth_account, bob.eth_account, eth_account].sort());
    }
  },
  {
    name: "Does not add auth without session keys",
    scenario: async ({ ctx, chain, starport, validators }) => {
      // spins up new validator charlie, doesnt add session keys, change validators should fail
      const keyring = ctx.actors.keyring;
      const peer_id = "12D3KooW9qtwBHeQryg9mXBVMkz4YivUsj62g1tYBACUukKToKof";
      const node_key = "0x0000000000000000000000000000000000000000000000000000000000000002";
      const eth_private_key = "0xb1b07e7078273a09c64ef9bd52f49636535ba26624c7c75a57e1286b13c8f7ea";
      const eth_account = "0x9c00B0af5586aE099649137ca6d00a641aD30736";

      await validators.addValidator("Charlie", { peer_id, node_key, eth_private_key, eth_account });

      const charlie = keyring.createFromUri("//Charlie");
      const charlieGatewayId = charlie.address;

      const { alice, bob } = validators.validatorInfoMap;
      const toValKeys = (substrateId, ethAccount) => {return  {substrate_id: keyring.decodeAddress(substrateId), eth_address: ethAccount} };
      const allAuthsRaw = [
        toValKeys(alice.aura_key, alice.eth_account),
        toValKeys(bob.aura_key, bob.eth_account),
        toValKeys(charlieGatewayId, eth_account),
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
