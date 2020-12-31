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

async function until(cond, opts={}) {
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
  let api, contracts, ps, web3;

  beforeEach(async () => {
    web3 = new Web3(ganache.provider(), null, { transactionConfirmationBlocks: 1 });
    contracts = await deployContracts(web3);
    let chainSpecFile = await buildChainSpec({
      name: 'Integration Test Network',
      properties: {
        eth_lock_event_topic,
        eth_starport_address: contracts.starport._address
      }
    }, false);

    let p2pPort = genPort();
    let wsPort = genPort();
    let rpcPort = genPort();

    // TODO: Point off-chain worker at our ganache provider
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
      p2pPort
    ]);

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
  });

  test('has the correct genesis hash', async () => {
    expect(api.genesisHash.toHex()).toBe('0x71ef780cdc604f5f974b25ccd02b9658712531c7e02217d0da260d24eabdbf7d');

    // TODO: Submit trx to Starport and check event logs

    // TODO: Submit extrinsic to Compound Chain and collect notices

    // TODO: Submit notices to Starport

    // TODO: Turn off validator
  }, 600000 /* 10m */);
});
