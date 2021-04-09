const { sleep } = require('../util');

class EventTracker {
  constructor(ctx) {
    this.trxId = 0;
    this.lastEvent = 0;
    this.allEvents = [];
    this.callbacks = [];
    this.ctx = ctx;
    this.unsubTimer = null;
    this.cancellers = [];
  }

  async subscribeEvents() {
    this.ctx.api().query.system.events((events) => {
      events.forEach(({ event, phase }, i) => {
        this.ctx.debug(`Found event: ${event.section}:${event.method} [${phase.toString()}]`);
      });
      // TODO: Clean this up [trying to remember why we sleep here...]
      this.ctx.sleep(5000).then(() => {
        // let finalizedEvents = events.filter(({phase}) => phase.Finalization);
        // debug(`Found ${finalizedEvents.length } finalized event(s)`);

        this.allEvents = [...this.allEvents, ...events];
        this.callbacks.forEach((callback) => callback(this.allEvents));
      });
    });
  }

  waitForEvent(pallet, method, opts = {}) {
    opts = {
      failureEvent: null,
      trackLastEvent: true,
      timeout: 30000,
      ...opts
    };

    let resolve, reject, timeoutId;
    let promise = new Promise((resolve_, reject_) => {
      if (opts.timeout) {
        timeoutId = setTimeout(() => {
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
          if (opts.trackLastEvent && i <= this.lastEvent) {
            return;
          }

          if (event.section === pallet && event.method === method) {
            if (opts.trackLastEvent) {
              this.lastEvent = i;
            }
            resolved = true;
            clearTimeout(timeoutId);
            return resolve(event);
          } else if (opts.failureEvent && event.section === opts.failureEvent[0] && event.method === opts.failureEvent[1]) {
            resolved = true;
            clearTimeout(timeoutId);
            return reject(new Error(`Found failure event ${event.section}:${event.method} - ${JSON.stringify(getEventData(event))}`));
          }
        });
      }
    };

    this.callbacks.push(handler);
    handler(this.allEvents);
    this.cancellers.push(() => reject('cancelled'));

    return promise;
  }

  setUnsubDelay() {
    // Note: we happily overwrite any existing value (since we only move forward)
    this.unsubTimer = Date.now() + 5000;
  }

  sendAndWaitForEvents(call, opts = {}) {
    opts = {
      onFinalize: true,
      rejectOnFailure: true,
      signer: null,
      ...opts
    };

    let api = this.ctx.api();

    return new Promise(async (resolve, reject) => {
      const  id = this.trxId++;
      const debugMsg = (msg) => {
        this.ctx.debug(`sendAndWaitForEvents[id=${id}] - ${msg}`);
      }

      const doResolve = async (events) => {
        this.setUnsubDelay();
        await unsub(); // Note: unsub isn't apparently working, but we are calling it

        let cashFailures = events
          .filter(({ event }) => api.events.cash.Failure.is(event))
          .map(({ event: { data: reason } }) => {
            this.ctx.debug(`sendAndWaitForEvents[id=${id}] - Failing call: ${JSON.stringify(call)} ${call.toString()}`);

            return new Error(`DispatchError[id=${id}]: ${reason.toString()}`);
          });

        let systemFailures = events
          .filter(({ event }) => api.events.system.ExtrinsicFailed.is(event))
          // we know that data for system.ExtrinsicFailed is
          // (DispatchError, DispatchInfo)
          .map(({ event: { data: [error, info] } }) => {
            this.ctx.debug(`sendAndWaitForEvents[id=${id}] - Failing call: ${JSON.stringify(call)} ${call.toString()}`);

            if (call.method && call.method.callIndex && call.method.callIndex.length === 2) {
              const [failModule, failExtrinsic] = call.method.callIndex;

              this.ctx.debug(`sendAndWaitForEvents[id=${id}] - Hint: check module #${failModule}'s #${failExtrinsic} extrinsic`);
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

  async teardown() {
    // Give time for unsubs before we exit, otherwise we get a teardown error from PolkadotJS
    this.cancellers.forEach((canceller) => canceller());
    if (this.unsubTimer) {
      let delta = this.unsubTimer - Date.now();
      if (delta > 0) {
        await sleep(delta); // This is an allowed teardown sleep
      }
    }
  }
}

async function buildEventTracker(ctx) {
  let eventTracker = new EventTracker(ctx);
  if (ctx.validators.count() > 0) {
    await eventTracker.subscribeEvents();
  }
  return eventTracker;
}

module.exports = {
  EventTracker,
  buildEventTracker
};
