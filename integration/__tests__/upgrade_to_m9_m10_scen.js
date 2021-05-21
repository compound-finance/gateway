const { buildScenarios } = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');
const { bytes32, encodeULEB128Hex } = require('../util/util');
const { getNotice } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 1000 } }
  ]
};

buildScenarios('Upgrade from m8 to m9 then to m10', scen_info, [
  {
    skip: true, // Currently CI doesnt have native binaries
    name: "Upgrade from m8 to m9 - m8 shell",
    info: {
      versions: ['m8', 'm9', 'm10'],
      genesis_version: 'm8',
      native: true,
      validators: {
        alice: {
          version: 'm8',
          extra_versions: ['m10'],
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
    scenario: async ({ api, alice, ashley, bob, chain, m9, m10, eth, keyring, sleep, starport, usdc, validators }) => {
      // First, do a lock
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(1);

      // Next, upgrade to m9
      await chain.upgradeTo(m9);

      expect(await chain.getSemVer()).toEqual([1, 9, 1]);

      // Lock again
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(2);

      await eth.updateGenesisBlock();

      // Now set Starport and Genesis Config
      await starport.executeProposal('Set Starport and Genesis Config', [
        alice.api.tx.cash.setStarport(starport.chainAddress()),
        alice.api.tx.cash.setGenesisBlock('Eth', eth.genesisBlock()),
      ]);

      await chain.upgradeTo(m10);

      expect(await chain.getSemVer()).toEqual([1, 10, 1]);

      // Lock a third USDC
      await ashley.lock(1, usdc);
      expect(await ashley.chainBalance(usdc)).toEqual(3);
    }
  }
]);
