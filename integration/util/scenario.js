const { buildCtx } = require('./scenario/ctx');
const { merge } = require('./util');

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
  if (Array.isArray(opts)) {
    scenarios = opts;
  }

  describe(name, () => {
    scenarios.forEach((scenario) => {
      let realTestFn = scenario.only ?
        test.only : ( scenario.skip ? test.skip : testFn );

      realTestFn(scenario.name, async () => {
        let scenInfo = merge(baseScenInfo, scenario.info || {});
        let ctx = await buildCtx(scenInfo);
        try {
          await scenario.scenario(ctx);
        } finally {
          await ctx.teardown();
        }
      }, scenario.timeout || 60000 /* 1m */);
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
};
