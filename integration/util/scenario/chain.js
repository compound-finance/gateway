const { waitForEvent } = require('../substrate');

class Chain {
  constructor(ctx) {
    this.ctx = ctx;
  }

  api() {
    return this.ctx.validators.api();
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

  async getNotices(event) {
    // TODO: How do we collect notices?
    throw new Error("Not implemented");
  }
}

function buildChain(ctx) {
  return new Chain(ctx);
}

module.exports = {
  buildChain,
  Chain
};
