const {
  buildScenarios
} = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');
const { bytes32 } = require('../util/util');
const { getNotice } = require('../util/substrate');

let scen_info = {};

buildScenarios('Upgrade to m8', scen_info, [
  {
    name: "Upgrade from m7 to m8",
    skip: true,
    info: {
      versions: ['m7'],
      genesis_version: 'm7',
      eth_opts: {
        version: 'm7',
      },
      validators: {
        alice: {
          version: 'm7',
        },
        bob: {
          version: 'm7',
        },
        charlie: {
          version: 'm7',
          eth_private_key: "0000000000000000000000000000000000000000000000000000000000000001" // Bad key
        }
      },
    },
    scenario: async ({ ctx, chain, validators, starport, curr, sleep }) => {
      const alice = validators.validatorInfoMap.alice;
      const bob = validators.validatorInfoMap.bob;
      const newAuthsRaw = [
        { substrate_id: ctx.actors.keyring.decodeAddress(alice.aura_key), eth_address: alice.eth_account },
        { substrate_id: ctx.actors.keyring.decodeAddress(bob.aura_key), eth_address: bob.eth_account }
      ];

      // Just set validators to same, but Bob won't be able to sign it
      let extrinsic = ctx.api().tx.cash.changeValidators(newAuthsRaw);

      let { notice } = await starport.executeProposal("Update authorities", [extrinsic], { awaitNotice: true });
      await chain.waitUntilSession(1);

      expect(await chain.noticeHold('Eth')).toEqual([1, 0]);

      let signatures = await chain.getNoticeSignatures(notice, { signatures: 2 });
      await starport.invoke(notice, signatures);
      await sleep(10000);

      expect(await chain.noticeState(notice)).toEqual({"Executed": null});
      expect(await chain.noticeHold('Eth')).toEqual([1, 0]);

      // Okay great, we've executed the change-over, but we still have a notice hold...
      // But what if we upgrade to m8??
      await chain.upgradeTo(curr);
      await chain.cullNotices();
      expect(await chain.noticeHold('Eth')).toEqual(null);

      // start at 0, rotate through 1, actually perform change over on 2
      await chain.waitUntilSession(2);
    }
  }
]);
