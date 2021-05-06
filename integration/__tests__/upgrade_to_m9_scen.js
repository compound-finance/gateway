const { buildScenarios } = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');
const { bytes32 } = require('../util/util');
const { getNotice } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } }
  ]
};

/* TODO: Replace curr with m9 once release is built */
buildScenarios('Upgrade to m9', scen_info, [
  {
    name: "Upgrade from m8 to m9 - current shell",
    info: {
      versions: ['m8'],
      genesis_version: 'm8',
      validators: {
        alice: {
          version: 'm8',
          extra_versions: ['curr'],
        },
        bob: {
          version: 'm8',
        },
        charlie: {
          version: 'm8',
          eth_private_key: "0000000000000000000000000000000000000000000000000000000000000001" // Bad key
        }
      },
    },
    scenario: async ({ api, alice, ashley, bob, chain, curr, keyring, sleep, starport, usdc, validators }) => {
      const newAuthsRaw = [
        { substrate_id: keyring.decodeAddress(alice.info.aura_key), eth_address: alice.info.eth_account },
        { substrate_id: keyring.decodeAddress(bob.info.aura_key), eth_address: bob.info.eth_account }
      ];

      // Just set validators to same, but Charlie won't be able to sign it
      let { notice: notice0 } = await starport.executeProposal(
        "Update authorities", [
          api.tx.cash.changeValidators(newAuthsRaw)
        ], { awaitNotice: true });
      await chain.waitUntilSession(1);
      expect(await chain.noticeHold('Eth')).toEqual([1, 0]);

      let signatures0 = await chain.getNoticeSignatures(notice0, { signatures: 2 });
      await starport.invoke(notice0, signatures0);
      await sleep(20000);
      expect(await chain.noticeState(notice0)).toEqual({"Executed": null});
      expect(await chain.noticeHold('Eth')).toEqual(null);
      expect(await chain.sessionValidators()).toEqualSet([alice.info.aura_key, bob.info.aura_key]);

      // Try to lock
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(1);

      // Rotate again
      let { notice: notice1 } = await starport.executeProposal(
        "Update authorities", [
          api.tx.cash.changeValidators(newAuthsRaw)
        ], { awaitNotice: true });
      await chain.waitUntilSession(2);
      expect(await chain.noticeHold('Eth')).toEqual([2, 0]);

      let signatures1 = await chain.getNoticeSignatures(notice1, { signatures: 2 });
      await starport.invoke(notice1, signatures1);
      await sleep(20000);
      expect(await chain.noticeState(notice1)).toEqual({"Executed": null});
      expect(await chain.noticeHold('Eth')).toEqual(null);
      expect(await chain.sessionValidators()).toEqualSet([alice.info.aura_key, bob.info.aura_key]);

      // Okay great, we've executed the change-over, but we still have a notice hold...
      // But what if we upgrade to curr??
      await chain.upgradeTo(curr);
      expect(await chain.getSemVer()).toEqual([1, 8, 1]);
      expect(await chain.noticeHold('Eth')).toEqual(null);

      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);

      // Try to lock again
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(2);
    }
  },
  {
    only: true, // Currently CI doesnt have native binaries
    name: "Upgrade from m8 to m9 - m8 shell",
    info: {
      versions: ['m8'],
      genesis_version: 'm8',
      native: true,
      validators: {
        alice: {
          version: 'm8',
          extra_versions: ['curr'],
        },
        bob: {
          version: 'm8',
        },
        charlie: {
          version: 'm8',
          eth_private_key: "0000000000000000000000000000000000000000000000000000000000000001" // Bad key
        }
      },
    },
    scenario: async ({ api, alice, ashley, bob, chain, curr, eth, keyring, sleep, starport, usdc, validators }) => {
      console.log("yabba0");
      const newAuthsRaw = [
        { substrate_id: keyring.decodeAddress(alice.info.aura_key), eth_address: alice.info.eth_account },
        { substrate_id: keyring.decodeAddress(bob.info.aura_key), eth_address: bob.info.eth_account }
      ];
      console.log("yabba1");
      // Just set validators to same, but Charlie won't be able to sign it
      let { notice: notice0 } = await starport.executeProposal(
        "Update authorities", [
          api.tx.cash.changeValidators(newAuthsRaw)
        ], { awaitNotice: true });
      await chain.waitUntilSession(1);
      expect(await chain.noticeHold('Eth')).toEqual([1, 0]);
      console.log("yabba2");
      let signatures0 = await chain.getNoticeSignatures(notice0, { signatures: 2 });
      await starport.invoke(notice0, signatures0);
      await sleep(20000);
      expect(await chain.noticeState(notice0)).toEqual({"Executed": null});
      expect(await chain.noticeHold('Eth')).toEqual(null);
      expect(await chain.sessionValidators()).toEqualSet([alice.info.aura_key, bob.info.aura_key]);
      console.log("yabba3");
      // Try to lock
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(1);
      console.log("yabba4");
      // Rotate again
      let { notice: notice1 } = await starport.executeProposal(
        "Update authorities", [
          api.tx.cash.changeValidators(newAuthsRaw)
        ], { awaitNotice: true });
      await chain.waitUntilSession(2);
      expect(await chain.noticeHold('Eth')).toEqual([2, 0]);
      console.log("yabba5");
      let signatures1 = await chain.getNoticeSignatures(notice1, { signatures: 2 });
      await starport.invoke(notice1, signatures1);
      await sleep(20000);
      expect(await chain.noticeState(notice1)).toEqual({"Executed": null});
      expect(await chain.noticeHold('Eth')).toEqual(null);
      expect(await chain.sessionValidators()).toEqualSet([alice.info.aura_key, bob.info.aura_key]);
      console.log("yabba6");
      // Okay great, we've executed the change-over, but we still have a notice hold...
      // But what if we upgrade to curr??
      let blockNumberHex = '0x' + ([...new Array(16)].map((i) => "0").join("") + eth.blockInfo.number.toString(16)).slice(-16);

      console.log([starport.ethAddress(), eth.blockInfo.hash, eth.blockInfo.parent_hash, blockNumberHex]);

      let enc = eth.web3.utils.asciiToHex;

      await chain.upgradeTo(curr, [
        //api.tx.cash.setGenesisBlock(eth.genesisBlock()),
        //api.tx.cash.setStarport(starport.chainAddress())
      ], (wasm) => {
        return wasm
          .replace(enc("0x7777777777777777777777777777777777777777"), enc(starport.ethAddress()))
          .replace(enc("0x8888888888888888888888888888888888888888888888888888888888888888"), enc(eth.blockInfo.hash))
          .replace(enc("0x9999999999999999999999999999999999999999999999999999999999999999"), (eth.blockInfo.parent_hash))
          .replace(enc("0xaaaaaaaaaaaaaaaa"), enc(blockNumberHex));

      });

      expect(await chain.getSemVer()).toEqual([1, 9, 1]);
      expect(await chain.noticeHold('Eth')).toEqual(null);
      console.log("yabba7");
      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);
      console.log("yabba8");
      // Try to lock again
      await ashley.lock(1, usdc);
      console.log("yabba9");
      expect(await ashley.chainBalance(usdc)).toEqual(2);
    }
  }
]);
