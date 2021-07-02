const { findEvent, getEventData, mapToJson, signAndSend } = require('../substrate');
const { arrayEquals, keccak256, intervalToSeconds, zip } = require('../util');
const {
  getNoticeChainId,
  encodeNotice,
  getNoticeParentHash,
  getNoticeId,
  getRawHash,
} = require('./types');
const chalk = require('chalk');

const { u8aToHex } = require('@polkadot/util');
const { xxhashAsHex } = require('@polkadot/util-crypto');
const web3 = require('web3');

function hexByte(x) {
  return ('00' + x.toString(16)).slice(-2);
}

class Chain {
  constructor(viaApi, ctx) {
    this.viaApi = viaApi;
    this.ctx = ctx;
  }

  via(validatorOrApi) {
    let viaApi = validatorOrApi.api ? validatorOrApi.api : validatorOrApi;
    return new Chain(viaApi, this.ctx)
  }

  toSS58 (arr) {
    return this.ctx.actors.keyring.encodeAddress(new Uint8Array(arr.buffer));
  }

  toEthAddress(u8arr) {
    return web3.utils.toChecksumAddress(u8aToHex(u8arr));
  }

  // https://www.shawntabrizi.com/substrate/querying-substrate-storage-via-rpc/
  getStorageKey(moduleName, valueName) {
    let moduleHash = xxhashAsHex(moduleName, 128);
    let functionHash = xxhashAsHex(valueName, 128);
    return moduleHash + functionHash.slice(2);
  }

  api() {
    return this.viaApi || this.ctx.getApi();
  }

  async waitForEvent(pallet, eventName, onFinalize = true, failureEvent = null) {
    return await this.ctx.eventTracker.waitForEvent(pallet, eventName, { failureEvent });
  }

  // Similar to wait for event, but will reject if it sees a `cash:FailedProcessingChainBlockEvent` event
  async waitForEthProcessEvent(pallet, eventName, onFinalize = true, autoMine = true) {
    return this.waitForL1ProcessEvent(this.ctx.eth, pallet, eventName, onFinalize, autoMine)
  }

  // Similar to wait for event, but will reject if it sees a `cash:FailedProcessingChainBlockEvent` event
  async waitForL1ProcessEvent(layer1, pallet, eventName, onFinalize = true, autoMine = true) {
    if (autoMine && this.ctx.__blockTime() === null) {
      await layer1.mine(100); // Just mine some blocks if we're waiting on an eth event to make things move
    }
    return this.waitForEvent(pallet, eventName, { failureEvent: ['cash', 'FailedProcessingChainBlockEvent'] });
  }

  async waitForEthProcessFailure(onFinalize = true) {
    return this.waitForEvent('cash', 'FailedProcessingEthEvent');
  }

  async waitForChainProcessed(onFinalize = true, failureEvent = null) {
    // TODO: Match transaction id?
    return await this.waitForEvent('cash', 'ProcessedChainBlockEvent', { failureEvent });
  }

  async waitForNotice(onFinalize = true, failureEvent = null) {
    return getEventData(await this.waitForEvent('cash', 'Notice', { failureEvent }));
  }

  async freezeTime(time) {
    await Promise.all(this.ctx.validators.all().map(async (validator) => {
      await validator.freezeTime(time);
    }));
  }

  async accelerateTime(interval, awaitBlock = true) {
    await Promise.all(this.ctx.validators.all().map(async (validator) => {
      let newTime = await validator.accelerateTime(1000 * intervalToSeconds(interval));
      if (awaitBlock) {
        await this.ctx.until(async () => {
          let currentTime = await validator.currentTime();
          return currentTime === newTime
        }, { delay: 1000 });
      }
    }));
  }

  async setFixedRate(token, bps) {
    let newModel = {
      Fixed: {
        rate: bps,
      }
    };
    let extrinsic = this.ctx.getApi().tx.cash.setRateModel(token.toChainAsset(), newModel);
    await this.ctx.starport.executeProposal("Update TokenRate Model", [extrinsic]);
  }

  async getNoticeChain(notice) {
    // We're going to walk back from the latest notice, tracking
    // the last accepted and a chain since that notice
    let chainId = getNoticeChainId(notice);
    let targetHash = keccak256(notice.EncodedNotice);

    let [currNoticeId, currChainHash] = (await this.api().query.cash.latestNotice(chainId)).toJSON();
    let currHash = getRawHash(currChainHash);
    let currChain = [];

    while (currNoticeId) {
      let currNotice = (await this.api().query.cash.notices(chainId, currNoticeId)).toJSON();

      if (arrayEquals(currNoticeId, notice.NoticeId)) {
        return currChain;
      }

      let encodedNotice = encodeNotice(currNotice);
      let parentHash = getNoticeParentHash(currNotice);
      let isAccepted = await this.ctx.starport.isNoticeInvoked(currHash);

      if (isAccepted) {
        currChain = [encodedNotice];
      } else {
        currChain = [encodedNotice, ...currChain];
      }

      currNoticeId = (await this.api().query.cash.noticeHashes({ [chainId]: parentHash })).toJSON();
      currHash = parentHash;
    }

    throw new Error(`Notice not found in notice chain`);
  }

  async getNoticeSignatures(notice, opts = {}) {
    opts = {
      sleep: 3000,
      retries: 20,
      signatures: await this.ctx.validators.quorum(),
      ...opts
    };
    let chainId = getNoticeChainId(notice);
    let noticeState = await this.api().query.cash.noticeStates(chainId, notice.NoticeId);
    if (!noticeState.isPending) {
      throw new Error("Unexpected notice status (not pending)");
    }
    let noticeStatePending = noticeState.asPending;

    let signaturePairs = noticeStatePending.signature_pairs.toJSON();

    if (!signaturePairs.eth && !signaturePairs.matic) {
      throw new Error("Unexpected signature pairs (not eth or matic)");
    }
    let pairs;
    if (signaturePairs.eth) {
      pairs = signaturePairs.eth;
    } else if (signaturePairs.matic) {
      pairs = signaturePairs.matic;
    }

    if (pairs.length < opts.signatures) {
      if (opts.retries > 0) {
        await this.ctx.sleep(opts.sleep);
        return await this.getNoticeSignatures(notice, { ...opts, retries: opts.retries - 1 });
      } else {
        throw new Error(`Unable to get signed notice in sufficient retries`);
      }
    } else {
      return pairs;
    }
  }

  async postPrice(payload, signature, onFinalize = true) {
    return await this.ctx.eventTracker.sendAndWaitForEvents(this.api().tx.oracle.postPrice(payload, signature), { onFinalize });
  }

  async cashIndex() {
    return await this.ctx.getApi().query.cash.globalCashIndex();
  }

  async upgradeTo(version, extrinsics = [], wasmFn = null) {
    this.ctx.log(chalk.blueBright(`Upgrading Chain to version ${version.version}...`));
    let versionHash = await version.hash();
    let allExtrinsics =
      [ this.ctx.getApi().tx.cash.allowNextCodeWithHash(versionHash)
      , ...extrinsics
      ];

    await this.ctx.starport.executeProposal(`Upgrade Chain to ${version.version}`, allExtrinsics);
    expect(await this.nextCodeHash()).toEqual(versionHash);
    let wasm = await version.wasm();
    await this.setNextCode(wasmFn ? wasmFn(wasm) : wasm, version, false);
    this.ctx.log(chalk.blueBright(`Upgrade to version ${version.version} complete.`));
  }

  async displayBlock() {
    const signedBlock = await this.ctx.getApi().rpc.chain.getBlock();

    // the information for each of the contained extrinsics
    signedBlock.block.extrinsics.forEach((ex, index) => {
      // the extrinsics are decoded by the API, human-like view
      this.ctx.log(index, ex.toHuman());

      const { isSigned, meta, method: { args, method, section } } = ex;

      // explicit display of name, args & documentation
      this.ctx.log(`${section}.${method}(${args.map((a) => a.toString()).join(', ')})`);
      this.ctx.log(meta.documentation.map((d) => d.toString()).join('\n'));

      // signer/nonce info
      if (isSigned) {
        this.ctx.log(`signer=${ex.signer.toString()}, nonce=${ex.nonce.toString()}`);
      }
    });
  }

  async tokenBalance(token, chainAccount) {
    let weiAmount = await this.api().query.cash.assetBalances(token.toChainAsset(), chainAccount);
    return token.toTokenAmount(weiAmount);
  }

  async interestRateModel(token) {
    let asset = await this.ctx.getApi().query.cash.supportedAssets(token.toChainAsset());
    return asset.unwrap().rate_model.toJSON();
  }

  async noticeHold(chainId) {
    return (await this.api().query.cash.noticeHolds(chainId)).toJSON();
  }

  async noticeState(notice) {
    let chainId = getNoticeChainId(notice);
    let noticeState = await this.api().query.cash.noticeStates(chainId, notice.NoticeId);
    return noticeState.toJSON();
  }

  async cullNotices() {
    return await this.ctx.eventTracker.sendAndWaitForEvents(this.api().tx.cash.cullNotices());
  }

  async nextCodeHash() {
    return mapToJson(await this.ctx.getApi().query.cash.allowedNextCodeHash());
  }

  async setNextCode(code, version, onFinalize = true) {
    await this.ctx.eventTracker.teardown();
    await this.ctx.eventTracker.send(this.api().tx.cash.setNextCodeViaHash(code), { setUnsubDelay: false, onFinalize: false });
    await Promise.all(this.ctx.validators.all().map((validator) => validator.teardownApi()));
    await this.ctx.sleep(60000);
    await Promise.all(this.ctx.validators.all().map((validator) => validator.setVersion(version)));

    await this.ctx.eventTracker.start();
  }

  async version() {
    return (await this.api().consts.system.version).toJSON();
  }

  async lastRuntimeUpgrade() {
    return mapToJson(await this.api().query.system.lastRuntimeUpgrade());
  }

  async getRuntimeVersion() {
    return (await this.api().rpc.state.getRuntimeVersion()).toJSON();
  }

  async getSemVer() {
    let semVer = await this.getRuntimeVersion();
    let {
      authoringVersion,
      specVersion,
      implVersion
    } = semVer;

    return [authoringVersion, specVersion, implVersion];
  }

  async pendingCashValidators() {
    let vals = await this.ctx.getApi().query.cash.nextValidators.entries();
    const authData = vals.map(([valIdRaw, chainKeys]) =>
      [
        this.toSS58(valIdRaw.args[0]),
        {eth_address: this.toEthAddress(chainKeys.unwrap().eth_address)}
      ]
    );
    return authData;
  }

  async cashValidators() {
    let vals = await this.ctx.getApi().query.cash.validators.entries();
    const authData = vals.map(([valIdRaw, chainKeys]) =>
      [
        this.toSS58(valIdRaw.args[0]),
        {eth_address: this.toEthAddress(chainKeys.unwrap().eth_address)}
      ]
    );
    return authData;
  }

  async sessionValidators() {
    let vals = await this.ctx.getApi().query.session.validators();
    return vals.map((valIdRaw) => this.toSS58(valIdRaw));
  }

  async getGrandpaAuthorities() {
    const grandpaStorageKey = ':grandpa_authorities';
    const grandpaAuthorities = await this.ctx.getApi().rpc.state.getStorage(grandpaStorageKey);
    let versionedAuthorities = this.ctx.getApi().createType('VersionedAuthorityList', grandpaAuthorities.unwrap());
    const authorityList = versionedAuthorities.authorityList;
    return authorityList.map(e => this.toSS58(e[0]));
  }

  async getAuraAuthorites() {
    const auraAuthStorageKey = this.getStorageKey("Aura", "Authorities");
    const rawAuths = await this.ctx.getApi().rpc.state.getStorage(auraAuthStorageKey);
    const auths = this.ctx.getApi().createType('Authorities', rawAuths.value);
    return auths.map(e => this.ctx.actors.keyring.encodeAddress(e));
  }

  async rotateKeys(validator) {
    const keysRaw = await validator.api.rpc.author.rotateKeys();
    return this.ctx.getApi().createType('SessionKeys', keysRaw);
  }

  async setKeys(signer, keys) {
    const call = this.ctx.getApi().tx.session.setKeys(keys, "0x5566");
    await this.ctx.eventTracker.sendAndWaitForEvents(call, { signer });
  }

  async newBlock() {
    return await this.ctx.eventTracker.newBlock();
  }

  async getBlockHeader() {
    return (await this.api().rpc.chain.getHeader()).toJSON();
  }

  async getBlockNumber() {
    let header = await this.getBlockHeader();
    return header.number;
  }

  async blocks(n) {
    const blockNum = await this.getBlockNumber();
    return await this.untilBlock(blockNum + n);
  }

  async untilBlock(number) {
    await this.ctx.until(async () => {
      const blockNum = await this.getBlockNumber();
      if (blockNum < number) {
        this.ctx.log(`Waiting for block=${number}, curr=${blockNum}`);
        return false;
      } else {
        return true;
      }
    }, { delay: 1000 });
  }

  async waitUntilSession(target) {
    await this.ctx.until(async () => {
      const idx = (await this.ctx.getApi().query.session.currentIndex()).toNumber();
      if (idx < target) {
        this.ctx.log(`Waiting for session=${target}, curr=${idx}`);
        return false;
      } else {
        return true;
      }
    }, { delay: 1000 });
  }

  async encodeCall(palletNumber, callNumber, version, argTypes, argValues) {
    let registry = await version.registry();
    let argsEncoded = zip(argTypes, argValues).map(([argType, argValue]) => {
      return registry.createType(argType, argValue).toHex().slice(2);
    });
    return `0x${hexByte(palletNumber)}${hexByte(callNumber)}${argsEncoded.join('')}`;
  }
}


function buildChain(ctx) {
  return new Chain(null, ctx);
}

module.exports = {
  buildChain,
  Chain
};
