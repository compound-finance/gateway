const ganache = require('ganache-core');
const Web3 = require('web3');
const { buildChainSpec, spawnValidator } = require('./validator');
const { deployContracts } = require('./ethereum');
const { log, error } = require('./log');
const { canConnectTo } = require('./net');
const { loadTypes } = require('./types');
const { genPort, sleep, until } = require('./util');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { Keyring } = require('@polkadot/api');

async function initialize(opts = {}) {
  try {
    let ganacheServer = ganache.server(opts.ganacheServer);
    let provider = ganacheServer.provider;

    let web3Port = opts.web3Port || genPort();

    // Start web3 server
    log(`Starting Ethereum server on ${web3Port}...`);
    ganacheServer.listen(web3Port);

    let web3 = new Web3(provider, null, { transactionConfirmationBlocks: 1 });
    let accounts = await web3.eth.personal.getAccounts();

    let contracts = await deployContracts(web3, ['0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61']);

    let starportTopics = Object.fromEntries(contracts
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
                "6a72a2f14577D9Cd0167801EFDd54a07B40d2b61"
              ]
            }
          }
        }
      }, false);
    } catch (e) {
      error("Failed to spawn validator node. Try running `cargo build --release`");
      throw e;
    }

    let rpcPort = opts.rpcPort || genPort();
    let p2pPort = opts.p2pPort || genPort();
    let wsPort = opts.wsPort || genPort();

    let logLevel = process.env['LOG'];
    let spawnOpts = logLevel ? { RUST_LOG: logLevel } : {};
    let extraArgs = logLevel ? [`-lruntime=${logLevel}`] : [];
    let ethPrivateKey = "50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374";

    let ps = spawnValidator([
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
      env: {
        ...spawnOpts,
        ETH_RPC_URL: `http://localhost:${web3Port}`,
        ETH_KEY_ID: ethPrivateKey
      }
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

    await until(() => canConnectTo('localhost', wsPort), {
      retries: 50,
      message: `awaiting websocket on port ${wsPort}...`
    });

    const wsProvider = new WsProvider(`ws://localhost:${wsPort}`);
    let api = await ApiPromise.create({
      provider: wsProvider,
      types: await loadTypes()
    });

    let keyring = new Keyring();
    let alice = keyring.addFromUri('//Alice');
    let bob = keyring.addFromUri('//Bob');

    return {
      accounts,
      alice,
      api,
      bob,
      contracts,
      ganacheServer,
      keyring,
      p2pPort,
      provider,
      ps,
      rpcPort,
      starportTopics,
      web3,
      web3Port,
      wsPort,
      wsProvider,
    };
  } catch (e) {
    error(`Test setup failed with error ${e}...`);
    error(e);
    process.exit(1);
  }
}

async function teardown({ api, ps, ganacheServer }) {
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
}

module.exports = {
  initialize,
  teardown
};
