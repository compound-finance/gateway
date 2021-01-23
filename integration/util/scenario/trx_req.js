const BigNumber = require('bignumber.js');
const { Token } = require('./token');
const { Actor } = require('./actor');

class TrxReq {
  constructor(ctx) {
    this.ctx = ctx;
  }

  toTrxArg(el) {
    if (typeof(el) === "string") {
      return el;
    } else if (typeof(el) === "number") {
      return el.toString();
    } else if (el instanceof BigNumber) {
      return el.toFixed();
    } else if (el instanceof Token) {
      return el.toTrxArg();
    } else if (el instanceof Actor) {
      return el.toTrxArg();
    }
  }

  generate(...args) {
    return `(${args.map(this.toTrxArg).join(' ')})`;
  }
}

function buildTrxReq(ctx) {
  return new TrxReq(ctx);
}

module.exports = {
  buildTrxReq,
  TrxReq,
};
