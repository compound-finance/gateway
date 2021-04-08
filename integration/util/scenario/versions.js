const { download } = require('../download');
const { instantiateInfo } = require('./scen_info');
const { keccak256 } = require('../util');
const fs = require('fs').promises;
const { constants } = require('fs');
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

async function pullVersion(repoUrl, version) {
  this.ctx.log(`Fetching version: ${version}...`);

  let wasmInfo = releaseWasmInfo(repoUrl, version);
  let typesInfo = releaseTypesInfo(repoUrl, version);
  let contractsInfo = releaseContractsInfo(repoUrl, version);

  await fs.mkdir(baseReleasePath(version), { recursive: true });

  await Promise.all([wasmInfo, typesInfo, contractsInfo].map(async ({ url, path }) => {
    this.ctx.log(`Downloading ${url} to ${path}`);
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
  }

  matches(v) {
    return this.version === v || this.symbolized === v;
  }

  releasePath() {
    return baseReleasePath(this.version);
  }

  async wasm() {
    let wasmBlob = await fs.readFile(this.wasmFile());
    return '0x' + wasmBlob.hexSlice();
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
    await pullVersion(this.ctx.__repoUrl(), this.version);
  }
}

class CurrentVersion extends Version {
  constructor(ctx) {
    super('curr', ctx);
  }

  async pull() {}

  wasmFile() {
    this.ctx.log({wasmFile: this.ctx.__wasmFile()});
    return this.ctx.__wasmFile();
  }

  typesJson() {
    return this.ctx.__typesFile();
  }

  contractsFile() {
    return this.ctx.__getContractsFile();
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
