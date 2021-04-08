const { buildCtx } = require('./scenario/ctx');
const { merge, sleep } = require('./util');
const os = require('os');
const path = require('path');

async function expectRevert(fn, reason) {
  // TODO: Expectation
  await fn();
}

function seconds(n) {
  return n * 1000;
}

function minutes(n) {
  return seconds(n * 60);
}

function hours(n) {
  return minutes(n * 60);
}

function days(n) {
  return hours(n * 24);
}

function years(n) {
  return days(n * 365);
}

function buildScenariosInternal(name, baseScenInfo, opts, scenarios, testFn) {
  if (Array.isArray(baseScenInfo)) {
    scenarios = baseScenInfo;
    baseScenInfo = {};
    opts = {};
  } else if (Array.isArray(opts)) {
    scenarios = opts;
    opts = {};
  }

  describe(name, () => {
    scenarios.forEach((scenario) => {
      let realTestFn = scenario.only ?
        test.only : (scenario.skip ? test.skip : testFn);

      realTestFn(scenario.name, async () => {
        let scenInfo = merge(baseScenInfo, scenario.info || {});
        if (process.env['QUIET_SCENARIOS']) {
          let scenFileName = `scenario-${name}-${scenario.name}`.replace(/[^a-zA-Z0-9-_]/g, '-');
          scenInfo.log_file = path.join(os.tmpdir(), scenFileName + '.log');
        }
        let ctx = await buildCtx(scenInfo);
        ctx.ctx = ctx; // Self reference to make ctx pattern-matchable for scenario fns
        try {
          let beforeFn = scenario.hasOwnProperty('before') ? scenario.before : opts.beforeEach;

          if (beforeFn) {
            await beforeFn(ctx);
          }
          await scenario.scenario(ctx);
        } finally {
          await ctx.teardown();
        }
      }, scenario.timeout || 600000 /* 10m */);
    });
  });
}

let buildScenarios = (name, baseScenInfo, opts, scenarios) =>
  buildScenariosInternal(name, baseScenInfo, opts, scenarios, test);

buildScenarios.skip = (name, baseScenInfo, opts, scenarios) =>
  buildScenariosInternal(name, baseScenInfo, opts, scenarios, test.skip);

buildScenarios.only = (name, baseScenInfo, opts, scenarios) =>
  buildScenariosInternal(name, baseScenInfo, opts, scenarios, test.only);

module.exports = {
  years,
  days,
  hours,
  minutes,
  seconds,
  merge,
  expectRevert,
  buildScenarios,
  sleep,
};
