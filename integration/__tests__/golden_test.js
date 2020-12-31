const ganache = require('ganache-core');
const Web3 = require('web3');
const { buildChainSpec, spawnValidator } = require('../util/validator');
const { deployContracts } = require('../util/ethereum');
const { log, error } = require('../util/log');
const { canConnectTo } = require('../util/net');
const { loadTypes } = require('../util/types');
const { genPort, sleep, until } = require('../util/util');
const { ApiPromise, WsProvider } = require('@polkadot/api');

describe('golden path', () => {
  let api, contracts, ganacheServer, provider, ps, web3;

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
  });

  test('has the correct genesis hash', async () => {
    expect(api.genesisHash.toHex()).toBe('0x71ef780cdc604f5f974b25ccd02b9658712531c7e02217d0da260d24eabdbf7d');

    // TODO: Submit trx to Starport and check event logs

    // TODO: Submit extrinsic to Compound Chain and collect notices

    // TODO: Submit notices to Starport

    // TODO: Turn off validator
  }, 600000 /* 10m */);
});
