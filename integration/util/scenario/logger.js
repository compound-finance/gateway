const fs = require('fs').promises;
const path = require('path');
const { debug, log, error } = require('../log');

class Logger {
  constructor(logFile, ctx) {
    this.logFile = logFile;
    this.logFD = null;
    this.ctx = ctx;
  }

  async openLogFile() {
    log("Writing scenario logs to " + this.logFile);
    if (this.logFile) {
      this.logFD = await fs.open(this.logFile, 'w');
    } else {
      return; // Don't log to file
    }
  }

  async logToFile(...msg) {
    if (this.logFD) {
      this.logFD.write(msg.map((m) => m.toString()).join("; ") + "\n");
    }
  }

  debug(...msg) {
    if (this.logFile) {
      this.logToFile(...msg);
    } else {
      debug(...msg);
    }
  }

  log(...msg) {
    if (this.logFile) {
      this.logToFile(...msg);
    } else {
      log(...msg);
    }
  }

  error(...msg) {
    if (this.logFile) {
      this.logToFile(...msg);
    } else {
      error(...msg);
    }
  }

  async teardown() {
    if (this.logFD) {
      // Clear before we close, since closing is async and we might not
      // be the next on the event loop for awhile.
      let logFD = this.logFD;
      this.logFD = null;
      await logFD.close();
    }
  }
}


async function buildLogger(ctx) {
  let logger = new Logger(ctx.logFile(), ctx);
  await logger.openLogFile();
  return logger;
}

module.exports = {
  Logger,
  buildLogger
};
