const { log, error } = require('./log');
const { buildCtx } = require('./scenario/ctx');

async function initialize(opts = {}) {
  try {
    let ctx = await buildCtx(opts);
    let testObject = ctx.getTestObject();

    let tokenStr = Object.entries(testObject.contracts.tokens).map(([name, contract]) => `${name}=${contract ? contract._address : null}`);
    log([
      `CashToken=${testObject.contracts.cashToken._address}`,
      `Starport=${testObject.contracts.starport._address}`,
      ...tokenStr
    ].join(', '));

    return testObject;
  } catch (e) {
    error(`Test setup failed with error ${e}...`);
    error(e);
    process.exit(1);
  }
}

async function teardown(ctx) {
  await ctx.teardown();
}

module.exports = {
  initialize,
  teardown
};
