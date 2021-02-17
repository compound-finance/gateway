const { sendAndWaitForEvents, waitForEvent, getEventData } = require('../substrate');
const { sleep, arrayEquals, keccak256 } = require('../util');
const { u8aToHex } = require('@polkadot/util');
const {
  getNoticeChainId,
  encodeNotice,
  getNoticeParentHash,
  getNoticeId,
  getRawHash,
} = require('./types');

class Chain {
  constructor(ctx) {
    this.ctx = ctx;
  }

  toSS58 (arr) {
    return this.ctx.actors.keyring.encodeAddress(new Uint8Array(arr.buffer));
  }

  api() {
    return this.ctx.api();
  }

  async waitForEvent(pallet, eventName, onFinalize = true, failureEvent = null) {
    return await waitForEvent(this.api(), pallet, eventName, onFinalize, failureEvent);
  }

  // Similar to wait for event, but will reject if it sees a `cash:FailedProcessingEthEvent` event
  async waitForEthProcessEvent(pallet, eventName, onFinalize = true) {
    return this.waitForEvent(pallet, eventName, onFinalize, ['cash', 'FailedProcessingEthEvent']);
  }

  async waitForEthProcessFailure(onFinalize = true) {
    return this.waitForEvent('cash', 'FailedProcessingEthEvent', onFinalize);
  }

  async waitForChainProcessed(onFinalize = true, failureEvent = null) {
    // TODO: Match transaction id?
    return await waitForEvent(this.api(), 'cash', 'ProcessedChainEvent', onFinalize, failureEvent);
  }

  async waitForNotice(onFinalize = true, failureEvent = null) {
    return getEventData(await waitForEvent(this.api(), 'cash', 'Notice', onFinalize, failureEvent));
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
      let isAccepted = await this.ctx.starport.isNoticeUsed(currHash);

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
      retries: 10,
      signatures: 2, // TODO: How many signatures do we want? We should ask the validator count? Or wait for Done?
      ...opts
    };
    let chainId = getNoticeChainId(notice);
    let noticeState = await this.api().query.cash.noticeStates(chainId, notice.NoticeId);
    if (!noticeState.isPending) {
      throw new Error("Unexpected notice status (not pending)");
    }
    let noticeStatePending = noticeState.asPending;

    let signaturePairs = noticeStatePending.signature_pairs;

    if (!signaturePairs.asEth) {
      throw new Error("Unexpected signature pairs (not eth)");
    }
    let signaturePairsEth = signaturePairs.asEth;
    let pairs = signaturePairsEth.map((k) => k);

    this.ctx.log(`Notice has ${pairs.length} signature pair(s)...`);

    if (pairs.length < opts.signatures) {
      if (opts.retries > 0) {
        await sleep(opts.sleep);
        return await this.getNoticeSignatures(notice, { ...opts, retries: opts.retries - 1 });
      } else {
        throw new Error(`Unable to get signed notice in sufficent retries`);
      }
    } else {
      return pairs;
    }
  }

  async postPrice(payload, signature, onFinalize = true) {
    return await sendAndWaitForEvents(this.api().tx.cash.postPrice(payload, signature), this.api(), onFinalize);
  }

  async cashIndex() {
    let index = await this.ctx.api().query.cash.globalCashIndex();
    return index.toNumber();
  }

  async interestRateModel(token) {
    let model = await this.ctx.api().query.cash.rateModels(token.toChainAsset());
    return model.toJSON();
  }


  async pendingCashValidators() {
    let vals = await this.ctx.api().query.cash.nextValidators.entries();
    const authData = vals.map(([valIdRaw, chainKeys]) =>
      [
        this.toSS58(valIdRaw.args[0]),
        {eth_address: u8aToHex(chainKeys.unwrap().eth_address)}
      ]
    );
    return authData;
  }

  async cashValidators() {
    let vals = await this.ctx.api().query.cash.validators.entries();
    const authData = vals.map(([valIdRaw, chainKeys]) =>
      [
        this.toSS58(valIdRaw.args[0]),
        {eth_address: u8aToHex(chainKeys.unwrap().eth_address)}
      ]
    );
    return authData;
  }

  async sessionValidators() {
    let vals = await this.ctx.api().query.session.validators();
  return vals.map((valIdRaw) => this.toSS58(valIdRaw));
  }
  
  async waitUntilSession(num) {
    const timer = ms => new Promise(res => setTimeout(res, ms));
    const checkIdx = async () => {
      const idx = (await this.ctx.api().query.session.currentIndex()).toNumber();
      if (idx <= num) {
        await timer(1000);
        await checkIdx();
      }
    };
    await checkIdx();
  }
}


function buildChain(ctx) {
  return new Chain(ctx);
}

module.exports = {
  buildChain,
  Chain
};
