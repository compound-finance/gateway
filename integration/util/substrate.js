const { debug, log } = require('./log');

let trxId = 0;

function waitForEvent(api, pallet, method, onFinalize = true) {
  return new Promise((resolve, reject) => {
    api.query.system.events((events) => {

      // Loop through the Vec<EventRecord>
      events.forEach(({ event }) => {
        debug(`Found event: ${event.section}:${event.method}`);
        if (event.section === pallet && event.method === method) {
          return resolve(event);
        }
      });
    });
  });
}

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
      } else if (status.isInvalid) {
        reject("Transaction failed (Invalid)");
      }
    }).then((unsub_) => unsub = unsub_);

    debugMsg(`Submitted unsigned transaction...`);
  });
}

function findEvent(events, pallet, method) {
  return events.find(({ event }) => event.section === pallet && event.method === method);
}

function getEventData(event) {
  if (event.event) { // Events are sometimes wrapped, let's make it easy for the caller
    event = event.event;
  }
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
  sendAndWaitForEvents,
  waitForEvent
};
