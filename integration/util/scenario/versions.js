const { download } = require('../download');
const { instantiateInfo } = require('./scen_info');
const { keccak256 } = require('../util');
const { constants } = require('fs');
const { TypeRegistry } = require('@polkadot/types');
const fs = require('fs').promises;
const path = require('path');
const os = require('os');

function releaseUrl(repoUrl, version, file) {
  return `${repoUrl}/releases/download/${version}/${file}`;
}

function baseReleasePath(version) {
  return path.join(__dirname, '..', '..', '..', 'releases', version);
}

function releasePath(version, file) {
  return path.join(baseReleasePath(version), file);
}

function releaseWasmInfo(repoUrl, version, v) {
  if (v.supports('new-artifacts')) {
    return {
      url: releaseUrl(repoUrl, version, 'gateway.wasm'),
      path: releasePath(version, 'gateway.wasm'),
    };
  } else {
    return {
      url: releaseUrl(repoUrl, version, 'gateway_runtime.compact.wasm'),
      path: releasePath(version, 'gateway_runtime.compact.wasm'),
    };
  }
}

function releaseTypesInfo(repoUrl, version, v) {
  return {
    url: releaseUrl(repoUrl, version, 'types.json'),
    path: releasePath(version, 'types.json'),
  };
}

function releaseContractsInfo(repoUrl, version, v) {
  return {
    url: releaseUrl(repoUrl, version, 'contracts.json'),
    path: releasePath(version, 'contracts.json'),
  };
}

function releaseTargetInfo(repoUrl, version, platform, arch, v) {
  return {
    url: releaseUrl(repoUrl, version, `gateway-${platform}-${arch}`),
    path: releasePath(version, `gateway-${platform}-${arch}`),
  };
}

async function pullVersion(ctx, repoUrl, version, v) {
  ctx.log(`Fetching version: ${version}...`);

  let wasmInfo = releaseWasmInfo(repoUrl, version, v);
  let typesInfo = releaseTypesInfo(repoUrl, version, v);
  let contractsInfo = releaseContractsInfo(repoUrl, version, v);
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

async function checkVersion(repoUrl, version, v) {
  let wasmInfo = releaseWasmInfo(repoUrl, version, v);
  let typesInfo = releaseTypesInfo(repoUrl, version, v);
  let contractsInfo = releaseContractsInfo(repoUrl, version, v);
  // TODO: Check target

  let exists = await Promise.all([wasmInfo, typesInfo, contractsInfo].map(async ({ url, path }) => {
    return checkFile(path);
  }));

  return exists.every((x) => x);
}

class Version {
  constructor(version, ctx) {
    this.version = version;
    this.ctx = ctx;
    this.symbolized = version.replace(/[.]/mig, '_');
    this.__registry = null;
    this.wasmReplacements = {};
  }

  name() {
    return this.version;
  }

  matches(v) {
    return this.version === v || this.symbolized === v;
  }

  releasePath() {
    return baseReleasePath(this.version);
  }

  async wasm() {
    let wasmBlob = await fs.readFile(this.wasmFile());
    let wasm = '0x' + wasmBlob.hexSlice();
    return Object.entries(this.wasmReplacements).reduce((acc, [k, v]) => {
      return acc.replace(new RegExp(k.slice(2), 'i'), v.slice(2));
    }, wasm);
  }

  wasmDir() {
    return path.dirname(this.wasmFile());
  }

  async hash() {
    return keccak256(await this.wasm());
  }

  wasmFile() {
    return releaseWasmInfo(this.ctx.__repoUrl(), this.version, this).path;
  }

  typesJson() {
    return releaseTypesInfo(this.ctx.__repoUrl(), this.version, this).path;
  }

  contractsFile() {
    return releaseContractsInfo(this.ctx.__repoUrl(), this.version, this).path;
  }

  targetFile() {
    return this.targetFileFor(os.platform(), os.arch());
  }

  targetFileFor(platform, arch) {
    return releaseTargetInfo(this.ctx.__repoUrl(), this.version, platform, arch, this).path;
  }

  async ensure() {
    let exists = await this.check();
    if (!exists) {
      await this.pull();
    }
  }

  async check() {
    return await checkVersion(this.ctx.__repoUrl(), this.version, this);
  }

  async pull() {
    await pullVersion(this.ctx, this.ctx.__repoUrl(), this.version, this);
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
      'starport-parent-block': (v) => v >= 9,
      'new-artifacts': (v) => v >= 10,
      'matic': (v) => v >= 12,
      'generic-cli-args': (v) => v >= 12,
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

  setWasmReplacements(wasmReplacements) {
    this.wasmReplacements = wasmReplacements;
  }
}

class CurrentVersion extends Version {
  constructor(ctx) {
    super('curr', ctx);
  }

  name() {
    return 'current';
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
    return this.ctx.__buildTarget();
  }

  isCurr() {
    return true;
  }

  async check() {
    if (!await checkFile(this.wasmFile())) {
      this.ctx.error(`Missing wasm file at ${this.wasmFile()}`)
    }

    if (!await checkFile(this.typesJson())) {
      this.ctx.error(`Missing types file at ${this.typesJson()}`)
    }

    if (!await checkFile(this.contractsFile())) {
      this.ctx.error(`Missing contracts file at ${this.contractsFile()}`)
    }

    return true;
  }

  versionNumber() {
    return 9999; // Arbitrarily high version number for "current"
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
