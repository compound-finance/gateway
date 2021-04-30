const { buildScenarios } = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');
const { bytes32 } = require('../util/util');
const { getNotice } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
};

buildScenarios('Upgrade to m4', scen_info, [
  {
    name: "Upgrade from m3 to m4",
    info: {
      versions: ['m3', 'm4'],
      genesis_version: 'm3',
      eth_opts: {
        version: 'm3',
      },
      validators: {
        alice: {
          version: 'm3',
        }
      },
    },
    scenario: async ({ ctx, ashley, zrx, chain, starport, cash, m4, sleep }) => {
      // Lock
      await ashley.lock(100, zrx);
      expect(await ashley.chainBalance(zrx)).toEqual(100);
      expect(await ashley.tokenBalance(zrx)).toEqual(900);

      // Then, upgrade the chain
      await chain.upgradeTo(m4);

      // Lock again
      await ashley.lock(200, zrx);
      expect(await ashley.chainBalance(zrx)).toEqual(300);
      expect(await ashley.tokenBalance(zrx)).toEqual(700);

      // Extract
      let notice = getNotice(await ashley.extract(50, zrx));
      let signatures = await chain.getNoticeSignatures(notice);
      await starport.invoke(notice, signatures);
      expect(await ashley.chainBalance(zrx)).toEqual(250);
      expect(await ashley.tokenBalance(zrx)).toEqual(750);

      // Next, upgrade the Cash Token to m4
      await cash.upgradeTo(m4);

      expect(await cash.getName()).toEqual('Cash');
      expect(await cash.getSymbol()).toEqual('CASH');
    }
  }
]);
