const util = require('util');
const child_process = require('child_process');
const { genPort, getInfoKey, until } = require('../util');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { canConnectTo } = require('../net');
const { instantiateInfo } = require('./scen_info');
const fs = require('fs').promises;
const path = require('path');
const chalk = require('chalk');

async function loadTypes(ctx) {
  let contents = await fs.readFile(ctx.__typesFile());
  try {
    return JSON.parse(contents);
  } catch (e) {
    let match = /in JSON at position (\d+)/.exec(e.message);
    if (match) {
      let pos = Number(match[1]);
      let show = (start, end) => contents.slice(start, end).toString().replaceAll("\n", "\\n");
      let colored =
        chalk.green(show(pos - 20, pos)) +
        chalk.red(show(pos, pos + 1)) +
        chalk.green(show(pos + 1, pos + 20));

      ctx.error("JSON Error Around: \n" + chalk.bgWhiteBright(colored));
      throw new Error(`Error Parsing \`types.json\`: ${e.toString()} [around \`${colored}\`]`);
    } else {
      throw new Error(`Error Parsing \`types.json\`: ${e.toString()}`);
    }
  }
}

let validatorInfoMap = {
  'alice': {
    aura_key: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    grandpa_key: "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu",
    eth_private_key: "50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374",
    eth_account: "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61",
    node_key: '0x0000000000000000000000000000000000000000000000000000000000000001',
    peer_id: '12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp', // I have _no idea_ how this is generated
    spawn_args: ['--alice'],
  },
  'bob': {
    aura_key: "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
    grandpa_key: "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E",
    eth_private_key: "6bc5ea78f041146e38233f5bc29c703c1cec8eaaa2214353ee8adf7fc598f23d",
    eth_account: "0x8ad1b2918c34ee5d3e881a57c68574ea9dbecb81",
    node_key: '0x0000000000000000000000000000000000000000000000000000000000000002',
    peer_id: '12D3KooWHdiAxVd8uMQR1hGWXccidmfCwLqcMpGwR6QcTP6QRMuD', // I have _no idea_ how this is generated
    spawn_args: ['--bob'],
  }
};

class Validator {
  constructor(ctx, name, info, rpcPort, p2pPort, wsPort, nodeKey, peerId, logLevel, spawnOpts, extraArgs, validatorArgs, ethPrivateKey, chainSpecFile) {
    this.ctx = ctx;
    this.name = name;
    this.info = info;
    this.rpcPort = rpcPort;
    this.p2pPort = p2pPort;
    this.wsPort = wsPort;
    this.nodeKey = nodeKey;
    this.peerId = peerId;
    this.logLevel = logLevel;
    this.spawnOpts = spawnOpts;
    this.extraArgs = extraArgs;
    this.validatorArgs = validatorArgs;
    this.ethPrivateKey = ethPrivateKey;
    this.chainSpecFile = chainSpecFile;
    this.wsProvider = null;
    this.api = null;
    this.ps = null;
    this.bootnodes = null;
  }

  asPeer() {
    // Note: we assume loopback address
    return `/ip4/127.0.0.1/tcp/${this.p2pPort}/p2p/${this.peerId}`;
  }

  async start(peers=[]) {
    this.bootnodes = peers.map((peer) => {
      return ['--reserved-nodes', peer];
    }).flat();

    let ps = spawnValidator(this.ctx, [
      '--chain',
      this.chainSpecFile,
      '--rpc-methods',
      'Unsafe',
      '--rpc-port',
      this.rpcPort,
      '--ws-port',
      this.wsPort,
      '--port',
      this.p2pPort,
      '--tmp',
      '--no-mdns',
      '--node-key',
      this.nodeKey,
      '-lruntime=debug',
      '--reserved-only',
      ...this.bootnodes,
      ...this.extraArgs,
      ...this.validatorArgs
    ], {
      env: {
        ...this.spawnOpts,
        ETH_RPC_URL: this.ctx.eth.web3Url,
        ETH_KEY: this.ethPrivateKey,
        ETH_KEY_ID: "my_eth_key_id"
      }
    });

    process.on('exit', () => {
      ps.kill('SIGTERM'); // No matter what, always kill compound-chain node
    });

    ps.on('error', (err) => {
      this.ctx.__abort(`Failed to spawn validator: ${err}`);
    });

    ps.on('close', (code) => {
      this.ctx.log(`Validator terminated, code=${code}`);
      if (code !== 0) {
        if (this.ctx.__linkValidator()) {
          this.ctx.__abort(`Validator failed unexpectedly with code ${code}`);
        }
      }
    });

    // TODO: Should we make awaiting optional? We could also spawn multiple at the
    //       same time, since this isn't order dependent.
    await until(() => canConnectTo('localhost', this.wsPort), {
      retries: 50,
      message: `Awaiting websocket for validator ${this.name} on port ${this.wsPort}...`
    });

    const wsProvider = new WsProvider(`ws://localhost:${this.wsPort}`);
    const api = await ApiPromise.create({
      provider: wsProvider,
      types: await loadTypes(this.ctx)
    });

    this.ps = ps;
    this.api = api;
    this.wsProvider = wsProvider;
  }

  async teardown() {
    if (this.api) {
      await this.api.disconnect(); // Disconnect from api
    }

    if (this.ps) {
      this.ps.kill('SIGTERM'); // Kill compound-chain node
    }
  }
}

class Validators {
  constructor(validators, ctx) {
    this.validators = validators;
    this.ctx = ctx;
  }

  all() {
    return this.validators;
  }

  first() {
    if (this.validators.length === 0) {
      throw new Error(`No validators for scenario`);
    } else {
      return this.validators[0];
    }
  }

  api() {
    return this.first().api;
  }

  get(name) {
    let validator = this.validators.find((validator) => validator.name === name);
    if (!validator) {
      throw new Error(`Unknown validator for scenario: ${name}`);
    } else {
      return validator;
    }
  }

  async start() {
    let peers = this.validators.map((validator) => validator.asPeer());
    await Promise.all(this.validators.map((validator) => validator.start(peers)));
  }

  async teardown() {
    await Promise.all(this.validators.map(async (validator) => {
      await validator.teardown();
    }));
  }
}

function spawnValidator(ctx, args = [], opts = {}) {
  ctx.log(`Starting validator node ${ctx.__target()} with args ${JSON.stringify(args)}`)

  let proc = child_process.spawn(ctx.__target(), args, opts);

  proc.stdout.on('data', (data) => {
    ctx.log(`stdout: ${data}`);
  });

  proc.stderr.on('data', (data) => {
    ctx.error(`stderr: ${data}`);
  });

  proc.on('close', (code) => {
    ctx.log(`child process exited with code ${code}`);
  });

  return proc;
}

function buildValidator(validatorName, validatorInfo, ctx) {
  ctx.log(`Starting Validator ${validatorName}...`);

  let rpcPort = validatorInfo.rpc_port || genPort();
  let p2pPort = validatorInfo.p2p_port || genPort();
  let wsPort = validatorInfo.ws_port || genPort();
  let nodeKey = getInfoKey(validatorInfo, 'node_key', `validator ${validatorName}`);
  let peerId = getInfoKey(validatorInfo, 'peer_id', `validator ${validatorName}`);

  let logLevel = ctx.__logLevel();
  let spawnOpts = logLevel !== 'info' ? { RUST_LOG: logLevel } : {};
  let extraArgs = logLevel !== 'info' ? [`-lruntime=${logLevel}`] : [];
  let validatorArgs = validatorInfo.spawn_args || [];

  let ethPrivateKey = getInfoKey(validatorInfo, 'eth_private_key', `validator ${validatorName}`);
  if (!ctx.chainSpec) {
    throw new Error(`Must initialize chain spec before starting validator`);
  }

  let chainSpecFile = ctx.chainSpec.file();

  return new Validator(ctx, validatorName, validatorInfo, rpcPort, p2pPort, wsPort, nodeKey, peerId, logLevel, spawnOpts, extraArgs, validatorArgs, ethPrivateKey, chainSpecFile);
}

async function getValidatorsInfo(validatorsInfoHash, ctx) {
  return await instantiateInfo(validatorsInfoHash, 'Validator', 'name', validatorInfoMap);
}

async function buildValidators(validatorsInfoHash, ctx) {
  ctx.log("Starting Validators...");

  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);
  let validatorsList = await validatorsInfo.map(([validatorName, validatorInfo]) =>
    buildValidator(validatorName, validatorInfo, ctx));

  let validators = new Validators(validatorsList, ctx);
  await validators.start();

  validators.validatorInfoMap = validatorInfoMap;
  return validators;
}

module.exports = {
  getValidatorsInfo,
  buildValidator,
  buildValidators,
  Validator,
  Validators,
};
