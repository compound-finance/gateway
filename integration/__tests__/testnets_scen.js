const { buildScenarios } = require('../util/scenario');

let startingBlock = 10268401;

buildScenarios('Test-nets', [
  {
    skip: true,
    name: "Connect to Ropsten Chain",
    info: {
      eth_opts: {
        provider: "https://ropsten-eth.compound.finance",
        block_number: startingBlock,
      },
      eth_supply_cap: null,
      validators: ['alice', 'bob', 'charlie', 'dave'],
      starport: {
        existing: {
          proxy_admin: '0xd418B3A7c4a2b9d60fF8dDf9E94a8b75Aa3f60A5',
          proxy: '0xD905AbBA1C5Ea48c0598bE9F3f8ae31290B58613',
          starport: '0xD905AbBA1C5Ea48c0598bE9F3f8ae31290B58613',
          starport_impl: '0xaa39fd81E66Eb9DbEEf3253319516A7317829Eb0',
        }
      },
      cash_token: {
        existing: {
          proxy_admin: '0xd418B3A7c4a2b9d60fF8dDf9E94a8b75Aa3f60A5',
          proxy: '0xc65a4A1855d314033530A29Ab993A1717879E5BF',
          cash_token: '0xc65a4A1855d314033530A29Ab993A1717879E5BF',
          cash_impl: '0x1ffe465b3c82499e1C637c02EFECD128B7B454CF',
        }
      },
      tokens: { /* TODO: Add the rest of the Roptsten tokens */
        zrx: {
          build: 'zrx.json',
          contract: 'ZRXToken',
          address: '0xc0e2d7d9279846b80eacdea57220ab2333bc049d',
          supply_cap: null
        }
      }
    },
    scenario: async ({ api, alice, ashley, bob, charlie, dave, chain, curr, eventTracker, eth, keyring, sleep, starport, usdc, validators, until }) => {
      // What we want is half of the nodes to vote A, and half to vote B, and then consolidate on A
      // So we're going to fake out two nodes and then try to reconcile by upgrading to the newest m10 code.

      /*** Test Setup ***/

      let start = Date.now();

      async function currentBlockNumber() {
        return ((await alice.api.query.cash.lastProcessedBlock('Eth')).toJSON()).eth.number;
      }

      let recentRopstenBlockNumber = 10368632;

      await until(() => currentBlockNumber() > recentRopstenBlockNumber, {
        retries: 50,
        message: async () => {
          let current = await currentBlockNumber();
          let elapsed = ( Date.now() - start ) / 1000;
          let processedBlocks = current - startingBlock;
          let blockRate = processedBlocks / elapsed;
          let remaining = ( recentRopstenBlockNumber - current ) / blockRate;
          return `Waiting for recent Ropsten block: startingBlock=${start}, currentBlock=${current}, processedBlocks=${processedBlocks}, finalBlock=${recentRopstenBlockNumber}, blockProcessRate=${blockRate}, expectedRemainingTime=${remaining}s`
        },
        console: true
      });
    }
  }
]);
