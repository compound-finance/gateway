const { log, error } = require('../util/log');

function genPort() {
  // TODO: Actually check port is free?
  return Math.floor(Math.random() * (65535 - 1024)) + 1024;
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function until(cond, opts = {}) {
  let options = {
    delay: 5000,
    retries: null,
    message: null,
    ...opts
  };

  let start = +new Date();

  if (await cond()) {
    return;
  } else {
    if (options.message) {
      log(options.message);
    }
    await sleep(options.delay + start - new Date());
    return await until(cond, {
      ...options,
      retries: options.retries === null ? null : options.retries - 1
    });
  }
}

module.exports = {
  genPort,
  sleep,
  until
};
