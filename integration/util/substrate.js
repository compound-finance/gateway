const { debug, log } = require('./log');

let trxId = 0;

function sendAndWaitForEvents(call, onFinalize = true) {
  return new Promise((resolve, reject) => {
    let unsub;
    let id = trxId++;
    let debugMsg = (msg) => {
      debug(() => `sendAndWaitForEvents[id=${id}] - ${msg}`);
    }

    call.send(({ events = [], status }) => {
      debugMsg(`Current status is ${status}`);

      if (status.isInBlock) {
        debugMsg(`Transaction included at blockHash ${status.asInBlock}`);
        if (!onFinalize) {
          unsub(); // Note: unsub isn't apparently working, but we _are_ calling it
          resolve(events);
        }
      } else if (status.isFinalized) {
        debugMsg(`Transaction finalized at blockHash ${status.asFinalized}`);
        if (onFinalize) {
          unsub();
          resolve(events);
        }
      }
    }).then((unsub_) => unsub = unsub_);

    debugMsg(`Submitted unsigned transaction...`);
  });
}

function findEvent(events, pallet, method) {
  return events.find(({ event }) => event.section === pallet && event.method === method);
}

function getEventData({ event }) {
  const types = event.typeDef;

  return event.data.reduce((acc, value, index) => {
    let key = types[index].type;
    debug(() => `getEventData: ${key}=${value.toString()}`);
    return {
      ...acc,
      [key]: value.toJSON()
    };
  }, {});
}

module.exports = {
  findEvent,
  getEventData,
  sendAndWaitForEvents
};
