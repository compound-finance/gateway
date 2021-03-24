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
    skip: true,
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

      // Uh-oh [TODO: Match real expectations]
      expect(await ashley.tokenBalance(usdc)).toEqual(0);
      expect(await ashley.chainBalance(usdc)).toEqual(100);
      expect(await starport.tokenBalance(usdc)).toEqual(0);
      expect(await bert.tokenBalance(usdc)).toEqual(100);
    }
  },
  {
    skip: true,
    name: 'Re-org with Identical Tx',
    scenario: async ({ ashley, bert, ether, chain, snapshot, starport, eth, sleep }) => {
      let crazyLock = await eth.__deploy('CrazyLock', [starport.ethAddress()]);
      let now = Date.now();
      let nowEven = now - (now % 2);
      let nowOdd = nowEven + 1;

      let snapshotId = await eth.snapshot();
      await eth.mine(1, nowEven);
      let tx0 = await crazyLock.methods.crazyLock(ashley.ethAddress()).send({value: 0.1e18, from: ashley.ethAddress()});
      await eth.mine(20);
      await chain.waitForEthProcessEvent('cash', ether.lockEventName());

      expect(await ashley.chainBalance(ether)).toEqual(0.1);
      expect(await starport.tokenBalance(ether)).toEqual(0.1);

      // Now it's time for the re-org
      await eth.restore(snapshotId);
      await eth.mine(1, nowOdd);
      let tx1 = await crazyLock.methods.crazyLock(ashley.ethAddress()).send({value: 0.1e18, from: ashley.ethAddress()});
      console.log({tx1});
      await eth.mine(20);
      await sleep(20000); // Give the chain time to process, even though nothing should have happened

      // Show that the transaction itself is the same
      expect(tx0.transactionHash).toEqual(tx1.transactionHash);
      expect(tx0.blockNumber).toEqual(tx1.blockNumber);
      expect(tx0.transactionIndex).toEqual(tx1.transactionIndex);

      // Uh-oh [TODO: Match real expectations]
      expect(await ashley.chainBalance(ether)).toEqual(0.1);
      expect(await starport.tokenBalance(ether)).toEqual(0);
    }
  }
]);
