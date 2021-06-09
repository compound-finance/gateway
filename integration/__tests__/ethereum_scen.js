const { buildScenarios } = require('../util/scenario');

let ethereum_scen_info = {
  tokens: []
};

buildScenarios('Ethereum Scenarios', ethereum_scen_info, [
  {
    name: "Locker: Lock and Extract",
    scenario: async ({ chain, zrx, eth, starport }) => {
      let locker = await eth.__deploy('Locker', [starport.ethAddress()]);
      await locker.methods.lockAndtransfer().send({ from: eth.root(), value: 0.1e18 });
      // This should, if all goes well, XXX
      let event = await chain.waitForEvent('cash', 'TransferCash');
      console.log({event});
    }
  }
]);
