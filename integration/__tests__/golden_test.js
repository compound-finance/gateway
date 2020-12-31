const ganache = require('ganache-core');
const Web3 = require('web3');
const { buildChainSpec, spawnValidator } = require('../util/validator');
const { deployContracts } = require('../util/ethereum');
const { log, error } = require('../util/log');
const { canConnectTo } = require('../util/net');
const { loadTypes } = require('../util/types');
const { ApiPromise, WsProvider } = require('@polkadot/api');

const eth_lock_event_topic = "0xec36c0364d931187a76cf66d7eee08fad0ec2e8b7458a8d8b26b36769d4d13f3";

function genPort() {
  // TODO: Actually check port is free?
  return Math.floor(Math.random() * (65535 - 1024)) + 1024;
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function until(cond, opts = {}) {
  let options = {
    delay: 5000,
    retries: null,
    message: null,
    ...opts
  };

  let start = +new Date();

  if (await cond()) {
    return;
  } else {
    if (options.message) {
      log(options.message);
    }
    await sleep(options.delay + start - new Date());
    return await until(cond, {
      ...options,
      retries: options.retries === null ? null : options.retries - 1
    });
  }
}

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
