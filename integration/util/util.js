const { log, error } = require('../util/log');
const Web3Utils = require('web3-utils');

function genPort() {
  // TODO: Actually check port is free?
  return Math.floor(Math.random() * (65535 - 1024)) + 1024;
}

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

function merge(x, y) {
  Object.entries(y).forEach(([key, val]) => {
    if (typeof (x[key]) === 'object' && typeof (val) === 'object' && !Array.isArray(x[key]) && x[key] !== null) {
      x[key] = merge(x[key], val);
    } else {
      x[key] = val;
    }
  });

  return x;
}

function getInfoKey(info, key, type) {
  if (!info.hasOwnProperty(key)) {
    throw new Error(`Expected key \`${key}\` in ${JSON.stringify(info)} for ${type}`);
  }

  return info[key];
}

function stripHexPrefix(str) {
  if (str.startsWith('0x')) {
    return str.slice(2);
  } else {
    return str;
  }
}

function lookupBy(cls, lookupKey, arr, lookup) {
  if (lookup instanceof cls) {
    return lookup;
  } else if (typeof (lookup) === 'string') {
    let el = arr.find((el) => el[lookupKey] === lookup);

    if (!el) {
      throw new Error(`Unknown ${cls.name} for scenario: ${lookup} [${cls.name}s: ${arr.map((el) => el[lookupKey]).join(', ')}]`);
    } else {
      return el;
    }
  } else {
    throw new Error(`Don't know how to lookup ${cls.name} from \`${JSON.stringify(lookup)}\``);
  }
}

function arrayEquals(a, b) {
  return Array.isArray(a) &&
    Array.isArray(b) &&
    a.length === b.length &&
    a.every((val, index) => val === b[index]);
}

// Concat `a` and `b` typed arrays of same type
function concatArray(a, b) {
    var c = new (a.constructor)(a.length + b.length);
    c.set(a, 0);
    c.set(b, a.length);
    return c;
}

let arrayToHex = (x) => Buffer.from(x).toString('hex');

function bytes32(x) {
  if (!x.startsWith("0x")) {
    x = Web3Utils.asciiToHex(x);
  }

  let padding = 66 - x.length;
  return x.toLowerCase() + [...new Array(padding)].map((i) => "0").join("");
}

module.exports = {
  arrayToHex,
  concatArray,
  genPort,
  sleep,
  merge,
  getInfoKey,
  stripHexPrefix,
  lookupBy,
  arrayEquals,
  keccak256: Web3Utils.keccak256,
  bytes32,
};
