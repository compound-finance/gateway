const { log, error } = require('../util/log');
const Web3Utils = require('web3-utils');
const util = require('util');

function inspect(...args) {
  console.log(util.inspect(args, { showHidden: false, depth: null }));
}

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

function intervalToSeconds(interval) {
  let intervalLengths = {
    second: 1,
    minute: 60,
    hour: 60 * 60,
    day: 24 * 60 * 60,
    month: 365 * 24 * 60 * 60 / 12,
    year: 365 * 24 * 60 * 60
  };

  return Object.entries(interval).reduce((acc, [k, v]) => {
    let unit = k.endsWith('s') ? k.slice(0, k.length - 1) : k;
    if (!intervalLengths.hasOwnProperty(unit)) {
      throw new Error(`Unknown time unit: ${k}`);
    }
    return acc + Math.ceil(intervalLengths[unit] * v);
  }, 0);
}

// From https://stackoverflow.com/a/22015930/320471
const zip = (a, b) => Array(Math.max(b.length, a.length)).fill().map((_,i) => [a[i], b[i]]);

const encodeULEB128 = (value, minLen = null) => {
  value |= 0n;
  const result = [];
  while (true) {
    const byte = value & 0x7fn;
    value >>= 7n;
    if (
      (minLen === null || result.length + 1 >= minLen) && (
        (value === 0n && (byte & 0x40n) === 0n) ||
        (value === -1n && (byte & 0x40n) !== 0n)
      )
    ) {
      result.push(byte);
      return result;
    }
    result.push(byte | 0x80n);
  }
};

function encodeULEB128Hex(value, minLen = null) {
  return '0x' + (encodeULEB128(value, minLen).map((x) => ('0' + x.toString(16)).slice(-2)).join(''));
}

module.exports = {
  arrayEquals,
  arrayToHex,
  bytes32,
  concatArray,
  genPort,
  getInfoKey,
  inspect,
  intervalToSeconds,
  keccak256: Web3Utils.keccak256,
  lookupBy,
  merge,
  sleep,
  stripHexPrefix,
  zip,
  encodeULEB128,
  encodeULEB128Hex
};
