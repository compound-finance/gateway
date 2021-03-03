const { debug, log } = require('./log');
const { arrayToHex, concatArray, sleep } = require('./util');
const types = require('@polkadot/types');

// TODO: Consider moving these vars into ctx
let trxId = 0;
let lastEvent = 0;

let subscribed;

let allEvents = [];
let callbacks = [];

// TODO: Refactor here?
function subscribeEvents(api) {
  api.query.system.events((events) => {
    events.forEach(({ event, phase }, i) => {
      debug(`Found event: ${event.section}:${event.method} [${phase.toString()}]`);
    });
    // TODO: Clean this up
    sleep(5000).then(() => {
      // let finalizedEvents = events.filter(({phase}) => phase.Finalization);
      // debug(`Found ${finalizedEvents.length } finalized event(s)`);

      allEvents = [...allEvents, ...events];
      callbacks.forEach((callback) => callback(allEvents));
    });
  });
}

function waitForEvent(api, pallet, method, opts = {}) {
  opts = {
    failureEvent: null,
    trackLastEvent: true,
    timeout: 20000,
    ...opts
  };

  if (!subscribed) {
    subscribeEvents(api);
    subscribed = true;
  }

  let resolve, reject;
  let promise = new Promise((resolve_, reject_) => {
    if (opts.timeout) {
      setTimeout(() => {
        reject_(new Error(`Timeout waiting for event ${pallet}:${method}`));
      }, opts.timeout);
    }
    resolve = resolve_;
    reject = reject_;
  });
  let resolved = false;
  let handler = (events) => {
    if (!resolved) {
      // Loop through the Vec<EventRecord>
      events.forEach(({ event }, i) => {
        if (opts.trackLastEvent && i <= lastEvent) {
          return;
        }

        if (event.section === pallet && event.method === method) {
          if (opts.trackLastEvent) {
            lastEvent = i;
          }
          resolved = true;
          return resolve(event);
        } else if (opts.failureEvent && event.section === opts.failureEvent[0] && event.method === opts.failureEvent[1]) {
          resolved = true;
          return reject(new Error(`Found failure event ${event.section}:${event.method} - ${JSON.stringify(getEventData(event))}`));
        }
      });
    }
  };

  callbacks.push(handler);
  handler(allEvents);

  return promise;
}

// TODO
async function signAndSend(call, signer) {
  const unsub = await call.signAndSend(signer, ({ status, events }) => {
    if (status.isInBlock || status.isFinalized) {
      debug(() => `signAndSend ${status}`);
      unsub();
    }
  });
}

function sendAndWaitForEvents(call, api, opts = {}) {
  opts = {
    onFinalize: true,
    rejectOnFailure: true,
    sender: null,
    ...opts
  };

  return new Promise(async (resolve, reject) => {
    const  id = trxId++;
    const debugMsg = (msg) => {
      debug(() => `sendAndWaitForEvents[id=${id}] - ${msg}`);
    }

    const doResolve = async (events) => {
      await unsub(); // Note: unsub isn't apparently working, but we are calling it

      let cashFailures = events
        .filter(({ event }) => api.events.cash.Failure.is(event))
        .map(({ event: { data: reason } }) => {
          debug(() => `sendAndWaitForEvents[id=${id}] - Failing call: ${JSON.stringify(call)} ${call.toString()}`);

          return new Error(`DispatchError[id=${id}]: ${reason.toString()}`);
        });

      let systemFailures = events
        .filter(({ event }) => api.events.system.ExtrinsicFailed.is(event))
        // we know that data for system.ExtrinsicFailed is
        // (DispatchError, DispatchInfo)
        .map(({ event: { data: [error, info] } }) => {
          debug(() => `sendAndWaitForEvents[id=${id}] - Failing call: ${JSON.stringify(call)} ${call.toString()}`);

          if (call.method && call.method.callIndex && call.method.callIndex.length === 2) {
            const [failModule, failExtrinsic] = call.method.callIndex;

            debug(() => `sendAndWaitForEvents[id=${id}] - Hint: check module #${failModule}'s #${failExtrinsic} extrinsic`);
          }

          if (error.isModule) {
            try {
              // for module errors, we have the section indexed, lookup
              const decoded = api.registry.findMetaError(error.asModule);
              const { documentation, method, section } = decoded;

              return new Error(`DispatchError[id=${id}]: ${section}.${method}: ${documentation.join(' ')}`);
            } catch (e) {}
          }

          // Other, CannotLookup, BadOrigin, no extra info
          return new Error(`DispatchError[id=${id}]: ${error.toString()}`);
        });

      let failures = [
        ...cashFailures,
        ...systemFailures
      ];

      if (opts.rejectOnFailure && failures.length > 0) {
        reject(failures[0]);
      } else {
        resolve(events);
      }
    };

    let doCall = opts.signer
      ? (cb) => call.signAndSend(opts.signer, cb)
      : (cb) => call.send(cb);

    const unsub = await doCall(({ events = [], status }) => {
      debugMsg(`Current status is ${status}`);

      if (status.isInBlock) {
        debugMsg(`Transaction included at blockHash ${status.asInBlock}`);
        if (!opts.onFinalize) {
          doResolve(events);
        }
      } else if (status.isFinalized) {
        debugMsg(`Transaction finalized at blockHash ${status.asFinalized}`);
        if (opts.onFinalize) {
          doResolve(events);
        }
      } else if (status.isInvalid) {
        reject("Transaction failed (Invalid)");
      }
    });

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

function descale(val, decimals) {
  return Number(`${val}e-${decimals}`);
}

function mapToJson(v) {
  if (v.isSome) {
    return v.unwrap().toJSON();
  } else {
    return null;
  }
}

module.exports = {
  decodeCall,
  encodeCall,
  descale,
  findEvent,
  getEventData,
  sendAndWaitForEvents,
  signAndSend,
  waitForEvent,
  getNotice,
  getEventName,
  mapToJson,
};
