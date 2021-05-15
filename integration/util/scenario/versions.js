const { download } = require('../download');
const { instantiateInfo } = require('./scen_info');
const { keccak256 } = require('../util');
const { constants } = require('fs');
const { TypeRegistry } = require('@polkadot/types');
const fs = require('fs').promises;
const path = require('path');

function releaseUrl(repoUrl, version, file) {
  return `${repoUrl}/releases/download/${version}/${file}`;
}

function baseReleasePath(version) {
  return path.join(__dirname, '..', '..', '..', 'releases', version);
}

function releasePath(version, file) {
  return path.join(baseReleasePath(version), file);
}

function releaseWasmInfo(repoUrl, version) {
  return {
    url: releaseUrl(repoUrl, version, 'gateway_runtime.compact.wasm'),
    path: releasePath(version, 'gateway_runtime.compact.wasm'),
  };
}

function releaseTypesInfo(repoUrl, version) {
  return {
    url: releaseUrl(repoUrl, version, 'types.json'),
    path: releasePath(version, 'types.json'),
  };
}

function releaseContractsInfo(repoUrl, version) {
  return {
    url: releaseUrl(repoUrl, version, 'contracts.json'),
    path: releasePath(version, 'contracts.json'),
  };
}

function releaseTargetInfo(repoUrl, version, platform, arch) {
  return {
    url: releaseUrl(repoUrl, version, `gateway-${platform}-${arch}`),
    path: releasePath(version, `gateway-${platform}-${arch}`),
  };
}

function* littleEndian(bigint, nbytes = 8) {
  for (let i = 0n; i < nbytes; i++) {
    yield (bigint >> (i * 8n)) & 255n;
  }
}

function numToWasmHex(num) {
  return [...littleEndian(BigInt(num))].reduce((a, b) => a + b.toString(16), '');
}

function wasmConfReplacer(conf) {
  console.log('xxx replacer', conf)
  return wasmHex => {
    console.log('xxx replacer called', conf, wasmHex.length)
    let hex = wasmHex;
    for (const name in conf) {
      const hash = keccak256(name);
      const value = conf[name];
      if ('u8x20' in value && value.u8x20.length == 42) {
        console.log('xxx u8x20', name, hash, value, hash.slice(2, 40), value.u8x20.slice(2), hex.match(hash.slice(2, 40)))
        //XXX hex = hex.replace(hash.slice(2, 40), value.u8x20.slice(2));
      } else if ('u8x32' in value && value.u8x32.length == 66) {
        console.log('xxx u8x32', name, hash, value, hash.slice(2, 64), value.u8x32.slice(2), hex.match(hash.slice(2, 64)))
        //XXX hex = hex.replace(hash.slice(2, 64), value.u8x32.slice(2));
      } else if ('u64' in value) {
        console.log('xxx u64', name, hash, value, numToWasmHex(hash), numToWasmHex(value.u64), hex.match(numToWasmHex(hash)))
        //XXX hex = hex.replace(numToWasmHex(hash), numToWasmHex(value.u64));
      } else {
        throw new Error(`Unexpected value for ${name}: ${JSON.stringify(value)}`)
      }
    }
    return hex;
  };
}

async function pullVersion(ctx, repoUrl, version) {
  ctx.log(`Fetching version: ${version}...`);

  let wasmInfo = releaseWasmInfo(repoUrl, version);
  let typesInfo = releaseTypesInfo(repoUrl, version);
  let contractsInfo = releaseContractsInfo(repoUrl, version);
  // TODO: Pull target

  await fs.mkdir(baseReleasePath(version), { recursive: true });

  await Promise.all([wasmInfo, typesInfo, contractsInfo].map(async ({ url, path }) => {
    ctx.log(`Downloading ${url} to ${path}`);
    await download(url, path);
  }));
}

async function checkFile(path) {
  try {
    await fs.access(path, constants.R_OK);
    return true;
  } catch (e) {
    return false;
  }
}

async function checkVersion(repoUrl, version) {
  let wasmInfo = releaseWasmInfo(repoUrl, version);
  let typesInfo = releaseTypesInfo(repoUrl, version);
  let contractsInfo = releaseContractsInfo(repoUrl, version);
  // TODO: Check target

  let exists = await Promise.all([wasmInfo, typesInfo, contractsInfo].map(async ({ url, path }) => {
    return checkFile(path);
  }));

  return exists.every((x) => x);
}

class Version {
  constructor(version, ctx, wasmFn = w => w) {
    this.version = version;
    this.ctx = ctx;
    this.wasmFn = wasmFn;
    this.symbolized = version.replace(/[.]/mig, '_');
    this.__registry = null;
  }

  matches(v) {
    return this.version === v || this.symbolized === v;
  }

  releasePath() {
    return baseReleasePath(this.version);
  }

  async wasm() {
    let wasmBlob = await fs.readFile(this.wasmFile());
    return this.wasmFn('0x' + wasmBlob.hexSlice());
  }

  async hash() {
    return keccak256(await this.wasm());
  }

  wasmFile() {
    return releaseWasmInfo(this.ctx.__repoUrl(), this.version).path;
  }

  typesJson() {
    return releaseTypesInfo(this.ctx.__repoUrl(), this.version).path;
  }

  contractsFile() {
    return releaseContractsInfo(this.ctx.__repoUrl(), this.version).path;
  }

  targetFile(platform, arch) {
    return releaseTargetInfo(this.ctx.__repoUrl(), this.version, platform, arch).path;
  }

  async ensure() {
    let exists = await this.check();
    if (!exists) {
      await this.pull();
    }
  }

  async check() {
    return await checkVersion(this.ctx.__repoUrl(), this.version);
  }

  async pull() {
    await pullVersion(this.ctx, this.ctx.__repoUrl(), this.version);
  }

  isCurr() {
    return false;
  }

  versionNumber() {
    let match = this.version.match(/^m(\d+)$/);

    if (match) {
      return Number(match[1]);
    }

    throw new Error(`No version number for ${this.version}`)
  }

  supports(t) {
    let versionMap = {
      'full-cli-args': (v) => v >= 9,
      'eth-starport-parent-block': (v) => v >= 9,
    };

    if (!versionMap.hasOwnProperty(t)) {
      throw new Error(`Unknown support type: ${t}`);
    }

    let versionCheck = versionMap[t];
    return versionCheck(this.versionNumber());
  }

  async loadTypes(ctx, version) {
    let contents = await fs.readFile(this.typesJson());
    return JSON.parse(contents);
  }

  async registry() {
    if (this.__registry) {
      return this.__registry;
    }

    let typesJson = await this.loadTypes()
    const registry = new TypeRegistry();
    registry.register(typesJson);
    this.__registry = registry;
    return registry;
  }

  withConf(conf) {
    return new Version(this.version, this.ctx, wasmConfReplacer(conf));
  }
}

class CurrentVersion extends Version {
  constructor(...args) {
    super('curr', ...args);
  }

  async pull() {}

  wasmFile() {
    return this.ctx.__wasmFile();
  }

  typesJson() {
    return this.ctx.__typesFile();
  }

  contractsFile() {
    return this.ctx.__getContractsFile();
  }

  targetFile(platform, arch) {
    return this.ctx.__target();
  }

  isCurr() {
    return true;
  }

  async check() {
    if (!await checkFile(this.wasmFile())) {
      this.ctx.warn(`Missing wasm file at ${this.wasmFile()}`)
    }

    if (!await checkFile(this.typesJson())) {
      this.ctx.warn(`Missing types file at ${this.typesJson()}`)
    }

    if (!await checkFile(this.contractsFile())) {
      this.ctx.warn(`Missing contracts file at ${this.contractsFile()}`)
    }

    return true;
  }

  versionNumber() {
    return 9999; // Arbitrarily high version number for "current"
  }

  withConf(conf) {
    return new CurrentVersion(this.ctx, wasmConfReplacer(conf));
  }
}

class Versions {
  constructor(versions, current, ctx) {
    this.versions = versions;
    this.current = current;
    this.ctx = ctx;
  }

  all() {
    return this.versions;
  }

  knownVersions() {
    return this.versions.map((v) => v.version);
  }

  find(version) {
    return this.all().find((v) => v.matches(version));
  }

  mustFind(version) {
    let v = this.find(version);
    if (!v) {
      throw new Error(`Unable to find version: ${version}, found: ${JSON.stringify(this.knownVersions())}`);
    }
    return v;
  }
}

async function buildVersion(version, ctx) {
  return new Version(version, ctx);
}

async function buildVersions(versionsInfo, ctx) {
  let versions = await versionsInfo.reduce(async (acc, versionInfo) => {
    if (versionInfo === 'curr') {
      // curr is automatically included
      return acc;
    }

    return [
      ...await acc,
      await buildVersion(versionInfo, ctx)
    ];
  }, Promise.resolve([]));

  let current = new CurrentVersion(ctx);
  versions.push(current);

  // Make sure we have all included versions
  await Promise.all(versions.map((version) => version.ensure()));

  return new Versions(versions, current, ctx);
}

module.exports = {
  buildVersions,
  buildVersion,
};
