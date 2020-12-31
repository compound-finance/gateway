var util = require('util');

function inspect(args) {
  if (args.length === 1) {
    if (typeof(args[0]) === 'string') {
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

function log(...args) {
  process.stdout.write(inspect(args));
}

module.exports = {
  error,
  log
};
