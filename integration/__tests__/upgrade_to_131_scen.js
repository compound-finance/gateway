const {
  buildScenarios
} = require('../util/scenario');
const { decodeCall, getEventData } = require('../util/substrate');

let scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
};

buildScenarios('Upgrade to 1.3.1', scen_info, [
  {
    name: "Upgrade from 1.2.1 to 1.3.1 with Live Events",
    info: {
      versions: ['v1.2.1'],
      genesis_version: 'v1.2.1',
      eth_opts: {
        version: 'v1.2.1',
      },
      validators: {
        alice: {
          version: 'v1.2.1',
        }
      },
    },
    scenario: async ({ ctx, ashley, zrx, chain, starport, curr, sleep }) => {
      // First, lock an asset in the Starport and check it
      let { tx, event } = await ashley.lock(100, zrx);
      expect(tx.events.Lock).toBeDefined(); // TODO: Deep check event
      expect(await ashley.chainBalance(zrx)).toEqual(100);

      // Then, upgrade the chain
      await chain.upgradeTo(curr);

      // Next, lock another asset in the Starport (Lock Old) and make sure it works
      ({ tx, event } = await ashley.lock(200, zrx));
      expect(tx.events.Lock).toBeDefined(); // TODO: Deep check event
      expect(await ashley.chainBalance(zrx)).toEqual(100);

      // Next, upgrade the Starport to 1.3.1
      // let newStarport = // ...;
      /// ...

      // Lock an asset (Lock New) and make sure it passes
      ({ tx, event } = await ashley.lock(200, zrx));
      expect(tx).toEqual(5); // TODO
    }
  }
]);
