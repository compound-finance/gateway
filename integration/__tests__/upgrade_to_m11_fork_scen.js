const { buildScenarios } = require('../util/scenario');

buildScenarios('Upgrade to m11 fork', [
  {
    only: true, // Currently CI doesnt have native binaries
    name: "Upgrade from m10 to m11 via fork",
    info: {
      versions: ['m10'],
      genesis_version: 'm10',
      native: true,
      validators: {
        alice: {
          version: 'm10'
        },
        bob: {
          version: 'm10'
        },
        charlie: {
          version: 'm10',
          eth_proxy: {
            hold: true
          },
        },
        dave: {
          version: 'm10',
          eth_proxy: {
            hold: true
          },
        }
      },
    },
    scenario: async ({ api, alice, ashley, bob, charlie, dave, chain, chainSpec, curr, eventTracker, m9, m10, eth, keyring, sleep, starport, usdc, validators }) => {
      // What we want is half of the nodes to vote A, and half to vote B, and then consolidate on A
      // So we're going to fake out two nodes and then try to reconcile by upgrading to the newest m10 code.

      /*** Test Setup ***/

      eth.stopMining();

      let block0 = await eth.getBlock(0);
      let block1 = await eth.getBlock(1);

      let badBlock1 = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
          "difficulty": "0x4ea3f27bc",
          "extraData": "0x476574682f4c5649562f76312e302e302f6c696e75782f676f312e342e32",
          "gasLimit": "0x1388",
          "gasUsed": "0x0",
          "hash": "0xdc0818cf78f21a8e70579cb46a43643f78291264dda342ae31049421c82d21ae",
          "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
          "miner": "0xbb7b8287f3f0a933474a79eae42cbca977791171",
          "mixHash": "0x4fffe9ae21f1c9e15207b1f472d5bbdd68c9595d461666602f2be20daf5e7843",
          "nonce": "0x689056015818adbe",
          "number": "0x1",
          "parentHash": block0.hash,
          "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
          "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
          "size": "0x220",
          "stateRoot": "0xddc8b0234c2e0cad087c8b389aa7ef01f7d79b2570bccb77ce48648aa61c904d",
          "timestamp": "0x55ba467c",
          "totalDifficulty": "0x78ed983323d",
          "transactions": [
          ],
          "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
          "uncles": [
          ]
        }
      };

      async function expectStuck(actor) {
        let lastProcessedBlock0 = (await actor.api.query.cash.lastProcessedBlock('Eth')).toJSON();
        let pendingChainBlocks0 = (await actor.api.query.cash.pendingChainBlocks('Eth')).toJSON();

        expect(lastProcessedBlock0.eth).toEqual({
          hash: block0.hash,
          parent_hash: '0x0000000000000000000000000000000000000000000000000000000000000000',
          number: 0,
          events: []
        });

        expect(pendingChainBlocks0[0]).toEqual({
          block: {
            eth: {
              hash: block1.hash,
              parent_hash: block0.hash,
              number: 1,
              events: []
            }
          },
          support: [
            '0x8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48',
            '0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d'
          ],
          dissent: [
            '0x306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20',
            '0x90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22'
          ]
        });
      }

      async function expectUnstuck(actor) {
        let lastProcessedBlock1 = (await actor.api.query.cash.lastProcessedBlock('Eth')).toJSON();
        let pendingChainBlocks1 = (await actor.api.query.cash.pendingChainBlocks('Eth')).toJSON();

        expect(lastProcessedBlock1.eth.number).toBeGreaterThan(5); // Expect progress
        expect(pendingChainBlocks1).toEqual([]); // Lots of progress
      }

      let proxyJson = (json) => {
        return (req, res, body) => {
          res.setHeader("Content-Type", "application/json");
          res.writeHead(200);
          res.end(JSON.stringify(json, null, 4));
        };
      };

      [charlie.ethProxy, dave.ethProxy].forEach((proxy) => {
        proxy.replace(/eth_getBlockByNumber.*\"0x1\"/, proxyJson(badBlock1));
        proxy.clearHold();
      });

      await Promise.all([
        alice.hardfork(curr),
        bob.hardfork(curr),
        charlie.hardfork(curr),
        dave.hardfork(curr),
      ]);

      /*** Actual Test ***/

      eth.startMining();

      await sleep(20000); // Allow some blocks to mine and get us "stuck"

      charlie.ethProxy.clear(); // Give Charlie and Dave the correct block #1 data
      dave.ethProxy.clear();

      await sleep(20000); // Let Charlie and Dave try to fix, but m10 bug prevents

      await expectStuck(alice); // Everyone is stuck
      await expectStuck(bob);
      await expectStuck(charlie);
      await expectStuck(dave);

      // TODO: Write a test where some nodes don't actually upgrade before the code substitute takes effect

      expect(await chain.via(alice).getSemVer()).toEqual([1, 10, 1]);
      expect(await chain.via(bob).getSemVer()).toEqual([1, 10, 1]);
      expect(await chain.via(charlie).getSemVer()).toEqual([1, 10, 1]);
      expect(await chain.via(dave).getSemVer()).toEqual([1, 10, 1]);

      console.log("getting block");
      let blockNumber = await chain.getBlockNumber();
      console.log({blockNumber});
      await chainSpec.addSubstitute(blockNumber + 10, curr);
      await chainSpec.generate();
      await chain.untilBlock(blockNumber + 20);

      console.log("checking unstuck");
      await expectUnstuck(alice);
      await expectUnstuck(bob);
      await expectUnstuck(charlie);
      await expectUnstuck(dave);

      expect(await chain.via(alice).getSemVer()).toEqual([1, 11, 1]);
      expect(await chain.via(bob).getSemVer()).toEqual([1, 11, 1]);
      expect(await chain.via(charlie).getSemVer()).toEqual([1, 11, 1]);
      expect(await chain.via(dave).getSemVer()).toEqual([1, 11, 1]);
    }
  }
]);
