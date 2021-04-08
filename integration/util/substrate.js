const { arrayToHex, concatArray } = require('./util');
const types = require('@polkadot/types');

function findEvent(events, pallet, method) {
  return events.find(({ event }) => event.section === pallet && event.method === method);
}

function getEventData(event) {
  if (event.event) { // Events are sometimes wrapped, let's make it easy for the caller
    event = event.event;
  }
  const typeDef = event.typeDef;

  return event.data.reduce((acc, value, index) => {
    let key = typeDef[index].type;
    // debug(() => `getEventData: ${key}=${value.toString()}`);
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
  getNotice,
  getEventName,
  mapToJson,
};
