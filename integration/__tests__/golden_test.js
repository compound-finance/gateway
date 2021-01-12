const ganache = require('ganache-core');
const Web3 = require('web3');
const { buildChainSpec, spawnValidator } = require('../util/validator');
const { deployContracts } = require('../util/ethereum');
const { log, error } = require('../util/log');
const { canConnectTo } = require('../util/net');
const { loadTypes } = require('../util/types');
const { genPort, sleep, until } = require('../util/util');
const { getEventData, findEvent, sendAndWaitForEvents, waitForEvent } = require('../util/substrate');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/api');

describe('golden path', () => {
  let
    accounts,
    alice,
    api,
    bob,
    contracts,
    ganacheServer,
    keyring,
    provider,
    ps,
    starportTopics,
    web3;

  beforeEach(async () => {
    try {
      ganacheServer = ganache.server();
      provider = ganacheServer.provider;

      let web3Port = genPort();

      // Start web3 server
      log(`Starting Ethereum server on ${web3Port}...`);
      ganacheServer.listen(web3Port);

      web3 = new Web3(provider, null, { transactionConfirmationBlocks: 1 });
      accounts = await web3.eth.personal.getAccounts();

      contracts = await deployContracts(web3);

      starportTopics = Object.fromEntries(contracts
        .starport
        ._jsonInterface
        .filter(e => e.type === 'event')
        .map(e => [e.name, e.signature]));

      let chainSpecFile;
      try {
        chainSpecFile = await buildChainSpec({
          name: 'Integration Test Network',
          properties: {
            eth_starport_address: contracts.starport._address,
            eth_lock_event_topic: starportTopics['Lock']
          },
          genesis: {
            runtime: {
              palletBabe: {
                authorities: [
                  // Use single well-known authority: Alice
                  [
                    "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
                    1
                  ]
                ]
              },
              palletCash: {
                validators: [
                  "04c3e5ff2cb194d58e6a51ffe2df490c70d899fee4cdfff0a834fcdfd327a1d1bdaae3f1719d7fd9a9ee4472aa5b14e861adef01d9abd44ce82a85e19d6e21d3a4"
                ]
              }
            }
          }
        }, false);
      } catch (e) {
        error("Failed to spawn validator node. Try running `cargo build --release`");
        throw e;
      }

      let rpcPort = genPort();
      let p2pPort = genPort();
      let wsPort = genPort();

      let logLevel = process.env['LOG'];
      let spawnOpts = logLevel ? { RUST_LOG: logLevel } : {};
      let extraArgs = logLevel ? [`-lruntime=${logLevel}`] : [];

      ps = spawnValidator([
        '--chain',
        chainSpecFile,
        '--rpc-methods',
        'Unsafe',
        '--rpc-port',
        rpcPort,
        '--ws-port',
        wsPort,
        '--port',
        p2pPort,
        '--tmp',
        '--alice',
        ...extraArgs
      ], {
        env: { ...spawnOpts, ETH_RPC_URL: `http://localhost:${web3Port}` }
      });

      ps.on('error', (err) => {
        error(`Failed to spawn validator: ${err}`);
        process.exit(1);
      });

      ps.on('close', (code) => {
        log(`Validator terminated, code=${code}`);
        if (code !== 0) {
          error(`Validator failed unexpectedly with code ${code}`);
          process.exit(1);
        }
      });

      // TODO: Fail on process error

      await until(() => canConnectTo('localhost', wsPort), {
        retries: 50,
        message: `awaiting websocket on port ${wsPort}...`
      });

      const wsProvider = new WsProvider(`ws://localhost:${wsPort}`);
      api = await ApiPromise.create({
        provider: wsProvider,
        types: await loadTypes()
      });

      keyring = new Keyring();
      alice = keyring.addFromUri('//Alice');
      bob = keyring.addFromUri('//Bob');
    } catch (e) {
      error(`Test setup failed with error ${e}...`);
      error(e);
      process.exit(1);
    }
  }, 600000 /* 10m */);

  afterEach(async () => {
    if (api) {
      await api.disconnect(); // Disconnect from api
    }

    if (ps) {
      ps.kill('SIGTERM'); // Kill compound-chain node
    }

    if (ganacheServer) {
      ganacheServer.close(); // Close Web3 server
    }

    await sleep(1000); // Give things a second to close
  });

  test('magic extraction', async () => {
    let call = api.tx.cash.magicExtract(
      [
        "Eth",
        "0xc00e94cb662c3520282e6f5717214004a7f26888"
      ], 1000);

    let events = await sendAndWaitForEvents(call, false);
    let magicExtractEvent = findEvent(events, 'cash', 'MagicExtract');

    expect(magicExtractEvent).toBeDefined();
    expect(getEventData(magicExtractEvent)).toEqual({
      GenericQty: 1000,
      GenericAccount: [
        "Eth",
        "0xc00e94cb662c3520282e6f5717214004a7f26888"
      ],
      Notice: {
        ExtractionNotice: {
          id: [expect.any(Number), 0],
          parent: "0x0000000000000000000000000000000000000000000000000000000000000000", "asset": "0x0000000000000000000000000000000000000000",
          account: "0xc00e94cb662c3520282e6f5717214004a7f26888",
          amount: 1000
        }
      }
    });

    // Everything's good.
  }, 600000 /* 10m */);

  test('lock asset', async () => {
    let tx = await contracts.starport.methods.lockETH().send({ value: 1e18, from: accounts[0] });
    let goldieLocksEvent = await waitForEvent(api, 'cash', 'GoldieLocks', false);

    expect(getEventData(goldieLocksEvent)).toEqual({
      "GenericAccount": [
        "Eth",
        accounts[0].toLowerCase(),
      ],
      "GenericAsset": [
        "Eth",
        "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      ],
      "GenericQty": "0x00000000000000000de0b6b3a7640000"
    });

    // Everything's good.
  }, 600000 /* 10m */);

  // TODO: Submit trx to Starport and check event logs

  // TODO: Submit extrinsic to Compound Chain and collect notices

  // TODO: Submit notices to Starport
});
