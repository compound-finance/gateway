const { log, error } = require('./log');

function sendAndWaitForEvents(call, onFinalize = true) {
  return new Promise((resolve, reject) => {
    let unsub;
    call.send(({ events = [], status }) => {
      log(`Current status is ${status}`);

      if (status.isInBlock) {
        log(`Transaction included at blockHash ${status.asInBlock}`);
        if (!onFinalize) {
          unsub();
          resolve(events);
        }
      } else if (status.isFinalized) {
        log(`Transaction finalized at blockHash ${status.asFinalized}`);
        if (onFinalize) {
          unsub();
          resolve(events);
        }
      }
    }).then((unsub_) => unsub = unsub_);
    log(`ZZZZ SENT`);
  });
}

module.exports = {
  sendAndWaitForEvents
}
