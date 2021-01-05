const ganache = require('ganache-core');
const Web3 = require('web3');
const { buildChainSpec, spawnValidator } = require('../util/validator');
const { deployContracts } = require('../util/ethereum');
const { log, error } = require('../util/log');
const { canConnectTo } = require('../util/net');
const { loadTypes } = require('../util/types');
const { genPort, sleep, until } = require('../util/util');
const { getEventData, findEvent, sendAndWaitForEvents } = require('../util/substrate');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/api');

describe('golden path', () => {
  let
    alice,
    api,
    bob,
    contracts,
    ganacheServer,
    keyring,
    provider,
    ps,
    web3;

  beforeEach(async () => {
    ganacheServer = ganache.server();
    provider = ganacheServer.provider;

    let web3Port = genPort();

    // Start web3 server
    log(`Starting Ethereum server on ${web3Port}...`);
    ganacheServer.listen(web3Port);

    web3 = new Web3(provider, null, { transactionConfirmationBlocks: 1 });

    contracts = await deployContracts(web3);
    let chainSpecFile = await buildChainSpec({
      name: 'Integration Test Network',
      properties: {
        eth_starport_address: contracts.starport._address
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
          }
        }
      }
    }, false);

    let rpcPort = genPort();
    let p2pPort = genPort();
    let wsPort = genPort();

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
      '--alice'
    ], {
      env: { ETH_RPC_URL: `http://localhost:${web3Port}` }
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
    let call = api.tx.cash.magicExtract({
      chain: "Eth",
      account: "0xc00e94cb662c3520282e6f5717214004a7f26888"
    }, "1000");

    let events = await sendAndWaitForEvents(call, false);
    let magicExtractEvent = findEvent(events, 'cash', 'MagicExtract');

    expect(magicExtractEvent).toBeDefined();
    expect(getEventData(magicExtractEvent)).toEqual({
      CashAmount: 1000,
      ChainAccount: {
        chain: "Eth",
        account: "0xc00e94cb662c3520282e6f5717214004a7f26888"
      },
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

  // TODO: Submit trx to Starport and check event logs

  // TODO: Submit extrinsic to Compound Chain and collect notices

  // TODO: Submit notices to Starport
});
