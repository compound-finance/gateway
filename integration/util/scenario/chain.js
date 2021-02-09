const { waitForEvent } = require('../substrate');
const { sleep } = require('../util');

class Chain {
  constructor(ctx) {
    this.ctx = ctx;
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

  async getNoticeSignatures(notice, opts = {}) {
    opts = {
      sleep: 3000,
      retries: 10,
      signatures: 2, // TODO: How many signatures do we want? We should ask the validator count? Or wait for Done?
      ...opts
    };

    let maybeNoticeStatus = await this.api().query.cash.noticeQueue([0, 0]);
    let noticeStatus = maybeNoticeStatus.unwrap();
    if (!noticeStatus.isPending) {
      throw new Error("Unexpected notice status (not pending)");
    }
    let noticeStatusPending = noticeStatus.asPending;

    let signaturePairs = noticeStatusPending.signature_pairs;

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
}

function buildChain(ctx) {
  return new Chain(ctx);
}

module.exports = {
  buildChain,
  Chain
};
