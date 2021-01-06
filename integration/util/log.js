var util = require('util');

function inspect(args) {
  if (args.length === 1) {
    if (typeof (args[0]) === 'string') {
      if (args[0].endsWith("\n")) {
        return args[0];
      } else {
        return args[0] + "\n";
      }
    } else {
      return util.inspect(args[0]) + "\n";
    }
  } else {
    return util.inspect(args) + "\n";
  }
}

function error(...args) {
  process.stderr.write(inspect(args));
}

function debug(...args) {
  if (!process.env['QUIET']) {
    let realArgs = args;
    if (args[0] && typeof (args[0].apply) === 'function') {
      realArgs = args[0].apply(null);
    }

    process.stdout.write(inspect(realArgs));
  }
}

function log(...args) {
  process.stdout.write(inspect(args));
}

module.exports = {
  debug,
  error,
  log
};
