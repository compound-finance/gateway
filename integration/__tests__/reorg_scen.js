const {
  buildScenarios
} = require('../util/scenario');

let lock_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 100 } }
  ],
  validators: ['alice', 'bob']
};

buildScenarios('Chain Re-organization Scenarios', lock_scen_info, [
  {
    name: 'Re-org Lock Collateral via Snapshot',
    scenario: async ({ ashley, bert, usdc, chain, snapshot, starport, eth }) => {
      let snapshotId = await eth.snapshot();
      await ashley.lock(100, usdc);
      await eth.mine(20);

      // Normal every day scenario
      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
      expect(await starport.tokenBalance(usdc)).toEqual(100);
      expect(await bert.tokenBalance(usdc)).toEqual(0);

      // Now it's time for the re-org
      await eth.restore(snapshotId);
      await ashley.tokenTransfer(bert, 100, usdc);
      await eth.mine(20);

      // Uh-oh
      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
      expect(await starport.tokenBalance(usdc)).toEqual(0);
      expect(await bert.tokenBalance(usdc)).toEqual(100);
    }
  }
]);
