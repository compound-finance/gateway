const { debug, log } = require('./log');
const { arrayToHex, concatArray } = require('./util');
const types = require('@polkadot/types');

// TODO: Consider moving into ctx
let trxId = 0;

function waitForEvent(api, pallet, method, onFinalize = true, failureEvent = null) {
  return new Promise((resolve, reject) => {
    api.query.system.events((events) => {

      // Loop through the Vec<EventRecord>
      events.forEach(({ event }) => {
        debug(`Found event: ${event.section}:${event.method}`);
        if (event.section === pallet && event.method === method) {
          return resolve(event);
        } else if (failureEvent && event.section === failureEvent[0] && event.method === failureEvent[1]) {
          return reject(new Error(`Found failure event ${event.section}:${event.method} - ${JSON.stringify(getEventData(event))}`));
        }
      });
    });
  });
}

function signAndSend(call, signerPair, api, onFinalize = true, rejectOnFailure = true) {
  const callFn = (callback) => call.signAndSend(signerPair, callback);
  return sendAndHandleEvents(callFn, api);
}

function sendAndWaitForEvents(call, api, onFinalize = true, rejectOnFailure = true) {
  const callFn = (callback) => call.send(callback);
  return sendAndHandleEvents(callFn, api);
}

function sendAndHandleEvents(sendable, api, onFinalize = true, rejectOnFailure = true) {
  return new Promise((resolve, reject) => {
    let unsub;
    let id = trxId++;
    let debugMsg = (msg) => {
      debug(() => `sendAndWaitForEvents[id=${id}] - ${msg}`);
    }
    sendable(({ events = [], status }) => {
      debugMsg(`Current status is ${status}`);

      let doResolve = (events) => {
        unsub(); // Note: unsub isn't apparently working, but we are calling it

        let failures = events
          .filter(({ event }) =>
            api.events.system.ExtrinsicFailed.is(event)
          )
          // we know that data for system.ExtrinsicFailed is
          // (DispatchError, DispatchInfo)
          .map(({ event: { data: [error, info] } }) => {
            debug(() => `sendAndWaitForEvents[id=${id}] - Failing call: ${JSON.stringify(call)} ${call.toString()}`);

            if (call.method && call.method.callIndex && call.method.callIndex.length === 2) {
              let [failModule, failExtrinsic] = call.method.callIndex;

              debug(() => `sendAndWaitForEvents[id=${id}] - Hint: check module #${failModule}'s #${failExtrinsic} extrinsic`);
            }

            if (error.isModule) {
              // for module errors, we have the section indexed, lookup
              const decoded = api.registry.findMetaError(error.asModule);
              const { documentation, method, section } = decoded;

              return new Error(`DispatchError[id=${id}]: ${section}.${method}: ${documentation.join(' ')}`);
            } else {
              // Other, CannotLookup, BadOrigin, no extra info
              return new Error(`DispatchError[id=${id}]: ${error.toString()}`);
            }
          });
        if (rejectOnFailure && failures.length > 0) {
          reject(failures[0]);
        } else {
          resolve(events);
        }
      };
      if (status.isInBlock) {
        debugMsg(`Transaction included at blockHash ${status.asInBlock}`);
        if (!onFinalize) {
          doResolve(events);
        }
      } else if (status.isFinalized) {
        debugMsg(`Transaction finalized at blockHash ${status.asFinalized}`);
        if (onFinalize) {
          doResolve(events);
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

function getEventName(event) {
  return `${event.event.section}:${event.event.method}`;
}

function getNotice(events) {
  let noticeEvent = events.find(({ event }) => event.section === 'cash' && event.method === 'Notice');
  if (!noticeEvent) {
    throw new Error(`Notice event not found. Events: ${events.map(getEventName).join(', ')})`);
  }

  return getEventData(noticeEvent);
}

function encodeCall(call) {
  return '0x' + arrayToHex(concatArray(call.callIndex, call.data));
}

function decodeCall(api, callData) {
  let call = new types.GenericCall(api.registry, callData);

  return call.toHuman();
}

module.exports = {
  decodeCall,
  encodeCall,
  findEvent,
  getEventData,
  sendAndWaitForEvents,
  signAndSend,
  waitForEvent,
  getNotice,
  getEventName,
};
