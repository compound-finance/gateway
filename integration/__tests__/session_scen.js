const { buildScenarios } = require('../util/scenario');
const { decodeCall } = require('../util/substrate');

let session_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } }
  ],
  types: {
    'Balance': 'u64' // TODO: Check type
  }
};

function toValKeys(keyring, substrateId, ethAccount) {
  return {
    substrate_id: keyring.decodeAddress(substrateId),
    eth_address: ethAccount
  };
};

function subToHex(keyring, substrateId) {
  const bytes = keyring.decodeAddress(substrateId);
  return bytes.reduce((a, b) => a + b.toString(16).padStart(2, '0'), '0x');
}

buildScenarios('Session Scenarios', session_scen_info, [
  {
    name: "Remove Authority Node (Alice & Bob -> Alice)",
    scenario: async ({ api, alice, chain, keyring, starport, validators }) => {
      const newAuthoritiesRaw = [
        toValKeys(keyring, alice.info.aura_key, alice.info.eth_account)
      ];

      let extrinsic = api.tx.cash.changeValidators(newAuthoritiesRaw);
      await starport.executeProposal("Update Authorities", [extrinsic]);

      const expectedAuthorities = [[ alice.info.aura_key, { eth_address: alice.info.eth_account } ]];

      expect(await chain.pendingCashValidators()).toEqual(expectedAuthorities);

      // Start at session 0, rotate through session 1, actually performs change-over on session 2
      await chain.waitUntilSession(2);

      // Check a session and validator keys are set properly
      expect(await chain.cashValidators()).toEqual(expectedAuthorities);
      expect(await chain.sessionValidators()).toEqual([alice.info.aura_key]);
      expect(await chain.getGrandpaAuthorities()).toEqual([alice.info.grandpa_key]);
      expect(await chain.getAuraAuthorites()).toEqual([alice.info.aura_key]);
    }
  },

  {
    name: "Add New Authority with Session Keys",
    scenario: async ({ api, alice, ashley, bob, cash, chain, starport, usdc, validators, keyring }) => {
      // Spin up new validator Charlie and add to auth set
      const charlie = await validators.addValidator("Charlie", 'charlie');
      console.log({charlie});
      const charlieKeys = await chain.rotateKeys(charlie);

      const charlieSubstrateKey = keyring.createFromUri("//Charlie");
      const charlieSubstrateId = charlieSubstrateKey.address;

      // Get Charlie some CASH so he can set session keys
      await ashley.lock(100, usdc);
      await ashley.transfer(1.01, cash, `Gate:${subToHex(keyring, charlieSubstrateId)}`);

      await chain.setKeys(charlieSubstrateKey, [charlieKeys.aura, charlieKeys.grandpa]);

      const allAuthsRaw = [
        toValKeys(keyring, alice.info.aura_key, alice.info.eth_account),
        toValKeys(keyring, bob.info.aura_key, bob.info.eth_account),
        toValKeys(keyring, charlieSubstrateId, charlie.info.eth_account),
      ];

      const extrinsic = api.tx.cash.changeValidators(allAuthsRaw);
      const { notice } = await starport.executeProposal("Update authorities", [extrinsic], { awaitNotice: true });

      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths).toEqualSet([alice.info.aura_key, bob.info.aura_key, charlieSubstrateId]);

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths).toEqualSet([alice.info.aura_key, bob.info.aura_key, keyring.encodeAddress(charlieKeys.aura)]);

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths).toEqualSet([alice.info.grandpa_key, bob.info.grandpa_key, keyring.encodeAddress(charlieKeys.grandpa)]);

      let signatures = await chain.getNoticeSignatures(notice);
      let tx = await starport.invoke(notice, signatures);

      const starportAuths = await starport.getAuthorities();
      expect([...starportAuths]).toEqualSet([alice.info.eth_account, bob.info.eth_account, charlie.info.eth_account]);
    }
  },

  {
    name: "Does Not Add Authority without Session Keys",
    scenario: async ({ api, alice, bob, chain, starport, validators, keyring }) => {
      // Spins up new validator charlie; doesn't add session keys. Change validators should fail.
      const charlie = await validators.addValidator("Charlie", {
        peer_id: "12D3KooWSCufgHzV4fCwRijfH2k3abrpAJxTKxEvN1FDuRXA2U9x",
        node_key: "0x0000000000000000000000000000000000000000000000000000000000000003",
        eth_private_key: "0xb1b07e7078273a09c64ef9bd52f49636535ba26624c7c75a57e1286b13c8f7ea",
        eth_account: "0x9c00B0af5586aE099649137ca6d00a641aD30736",
        spawn_args: ['--charlie']
      });
      const charlieKeys = await chain.rotateKeys(charlie);

      const charlieSubstrateKey = keyring.createFromUri("//Charlie");
      const charlieSubstrateId = charlieSubstrateKey.address;

      const allAuthsRaw = [
        toValKeys(keyring, alice.info.aura_key, alice.info.eth_account),
        toValKeys(keyring, bob.info.aura_key, bob.info.eth_account),
        toValKeys(keyring, charlieSubstrateId, charlie.info.eth_account),
      ];

      const extrinsic = api.tx.cash.changeValidators(allAuthsRaw);
      let { event } = await starport.executeProposal("Update Authorities", [extrinsic], { checkSuccess: false });

      let [payload, govResult] = event.data[0][0];
      if (!govResult.isDispatchFailure) {
        expect(govResult.toJSON()).toBe(null);
      }

      await chain.newBlock();

      const newSessionAuths = await chain.sessionValidators();
      expect(newSessionAuths).toEqualSet([alice.info.aura_key, bob.info.aura_key]);

      const auraAuths = await chain.getAuraAuthorites();
      expect(auraAuths).toEqualSet([alice.info.aura_key, bob.info.aura_key]);

      const grandpaAuths = await chain.getGrandpaAuthorities();
      expect(grandpaAuths).toEqualSet([alice.info.grandpa_key, bob.info.grandpa_key]);
    }
  }
]);
