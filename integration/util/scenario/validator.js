const util = require('util');
const child_process = require('child_process');
const { genPort, getInfoKey, until } = require('../util');
const { ApiPromise, WsProvider } = require('@polkadot/api');
const { canConnectTo } = require('../net');
const { instantiateInfo } = require('./scen_info');
const fs = require('fs').promises;
const path = require('path');

async function loadTypes(ctx) {
  return JSON.parse(await fs.readFile(ctx.__typesFile()));
}

let validatorInfoMap = {
  'alice': {
    babe_key: "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    grandpa_key: "5FA9nQDVg267DEd8m1ZypXLBnvN7SFxYwV7ndqSYGiN9TTpu",
    eth_private_key: "50f05592dc31bfc65a77c4cc80f2764ba8f9a7cce29c94a51fe2d70cb5599374",
    eth_account: "0x6a72a2f14577D9Cd0167801EFDd54a07B40d2b61",
    spawn_args: ['--alice'],
  },
  'bob': {
    babe_key: "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty",
    grandpa_key: "5GoNkf6WdbxCFnPdAnYYQyCjAKPJgLNxXwPjwTh6DGg6gN3E",
    eth_private_key: "6bc5ea78f041146e38233f5bc29c703c1cec8eaaa2214353ee8adf7fc598f23d",
    eth_account: "0x8ad1b2918c34ee5d3e881a57c68574ea9dbecb81",
    spawn_args: ['--bob'],
  }
};

class Validator {
  constructor(name, info, rpcPort, p2pPort, wsPort, wsProvider, api, ps) {
    this.name = name;
    this.info = info;
    this.rpcPort = rpcPort;
    this.p2pPort = p2pPort;
    this.wsPort = wsPort;
    this.wsProvider = wsProvider;
    this.api = api;
    this.ps = ps;
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

async function buildValidator(validatorName, validatorInfo, ctx) {
  ctx.log(`Starting Validator ${validatorName}...`);

  let rpcPort = validatorInfo.rpcPort || genPort();
  let p2pPort = validatorInfo.p2pPort || genPort();
  let wsPort = validatorInfo.wsPort || genPort();

  let logLevel = ctx.__logLevel();
  let spawnOpts = logLevel !== 'info' ? { RUST_LOG: logLevel } : {};
  let extraArgs = logLevel !== 'info' ? [`-lruntime=${logLevel}`] : [];
  let validatorArgs = validatorInfo.spawn_args || [];

  let ethPrivateKey = getInfoKey(validatorInfo, 'eth_private_key', `validator ${validatorName}`);
  if (!ctx.chainSpec) {
    throw new Error(`Must initialize chain spec before starting validator`);
  }

  let chainSpecFile = ctx.chainSpec.file();

  let ps = spawnValidator(ctx, [
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
    '-lruntime=debug',
    ...extraArgs,
    ...validatorArgs
  ], {
    env: {
      ...spawnOpts,
      ETH_RPC_URL: ctx.eth.web3Url,
      ETH_KEY: ethPrivateKey,
      ETH_KEY_ID: "my_eth_key_id"
    }
  });

  ps.on('error', (err) => {
    ctx.__abort(`Failed to spawn validator: ${err}`);
  });

  ps.on('close', (code) => {
    ctx.log(`Validator terminated, code=${code}`);
    if (code !== 0) {
      if (ctx.__linkValidator()) {
        ctx.__abort(`Validator failed unexpectedly with code ${code}`);
      }
    }
  });

  // TODO: Should we make awaiting optional? We could also spawn multiple at the
  //       same time, since this isn't order dependent.
  await until(() => canConnectTo('localhost', wsPort), {
    retries: 50,
    message: `Awaiting websocket for validator ${validatorName} on port ${wsPort}...`
  });

  const wsProvider = new WsProvider(`ws://localhost:${wsPort}`);
  const api = await ApiPromise.create({
    provider: wsProvider,
    types: await loadTypes(ctx)
  });

  return new Validator(validatorName, validatorInfo, rpcPort, p2pPort, wsPort, wsProvider, api, ps);
}

async function getValidatorsInfo(validatorsInfoHash, ctx) {
  return await instantiateInfo(validatorsInfoHash, 'Validator', 'name', validatorInfoMap);
}

async function buildValidators(validatorsInfoHash, ctx) {
  ctx.log("Starting Validators...");

  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);
  let validators = await validatorsInfo.reduce(async (acc, [validatorName, validatorInfo]) => {
    return [
      ...await acc,
      await buildValidator(validatorName, validatorInfo, ctx)
    ];
  }, Promise.resolve([]));

  return new Validators(validators, ctx);
}

module.exports = {
  getValidatorsInfo,
  buildValidator,
  buildValidators,
  Validator,
  Validators,
};
