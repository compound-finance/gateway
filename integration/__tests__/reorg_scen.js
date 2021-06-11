const { buildScenarios } = require('../util/scenario');

let lock_scen_info = {
  tokens: [
    { token: 'usdc', balances: { ashley: 100 } }
  ],
  validators: ['alice', 'bob', 'charlie']
};

buildScenarios('Chain Re-organization Scenarios', lock_scen_info, [
  {
    name: 'Re-org Lock Collateral and Lock Different Amount',
    scenario: async ({ ashley, bert, usdc, chain, snapshot, starport, eth, logger }) => {
      let snapshotId = await eth.snapshot();
      await ashley.lock(100, usdc);
      await eth.mine(20);

      // Normal every day scenario
      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
      expect(await starport.tokenBalance(usdc)).toEqual(100);

      // Suddenly a re-org and Ashley _instead_ locks 9 USDC, not 100.
      await eth.restore(snapshotId);
      await ashley.lock(9, usdc);
      await chain.newBlock();

      // Check for results matching re-org
      expect(await ashley.tokenBalance(usdc)).toEqual(91);
      expect(await ashley.chainBalance(usdc)).toEqual(9);
      expect(await starport.tokenBalance(usdc)).toEqual(9);
    }
  },
  {
    name: 'Re-org Lock Collateral but Send Away Elsewhere',
    scenario: async ({ ashley, bert, usdc, chain, snapshot, starport, eth, logger }) => {
      let snapshotId = await eth.snapshot();
      await ashley.lock(100, usdc);
      await eth.mine(20);

      // Normal every day scenario
      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
      expect(await starport.tokenBalance(usdc)).toEqual(100);
      expect(await bert.tokenBalance(usdc)).toEqual(0);

      // Suddenly a re-org and Ashley _instead_ just sent the USDC to Bert
      await eth.restore(snapshotId);
      await ashley.tokenTransfer(bert, 100, usdc);
      await eth.mine(20);
      await chain.waitForEvent('cash', 'ReorgRevertLocked');

      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(0);
      expect(await starport.tokenBalance(usdc)).toEqual(0);
      expect(await bert.tokenBalance(usdc)).toEqual(100);
    }
  },
  {
    only: true, // TODO FIX SCEN
    name: 'Re-org with Identical Tx',
    scenario: async ({ ashley, bert, ether, chain, snapshot, starport, eth, sleep }) => {
      let crazyLock = await eth.__deploy('CrazyLock', [starport.ethAddress()]);

      let snapshotId = await eth.snapshot();
      let tx0 = await crazyLock.methods.crazyLock(ashley.ethAddress()).send({value: 0.1e18, from: ashley.ethAddress()});
      await eth.mine(20);
      await chain.waitForEthProcessEvent('cash', ether.lockEventName());

      expect(await ashley.chainBalance(ether)).toEqual(0.1);
      expect(await starport.tokenBalance(ether)).toEqual(0.1);

      // Now it's time for the re-org
      await eth.restore(snapshotId);
      let tx1 = await crazyLock.methods.halt().send({from: bert.ethAddress()});
      let tx2 = await crazyLock.methods.crazyLock(ashley.ethAddress()).send({value: 0.1e18, from: ashley.ethAddress()});
      await eth.mine(20);
      await chain.blocks(5);
      await chain.waitForEvent('cash', 'ReorgRevertLocked');

      // Show that the transaction itself is the same
      expect(tx0.transactionHash).toEqual(tx2.transactionHash);

      expect(await ashley.chainBalance(ether)).toEqual(0);
      expect(await starport.tokenBalance(ether)).toEqual(0);
    }
  }
]);
