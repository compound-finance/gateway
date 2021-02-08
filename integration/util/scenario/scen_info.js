
const baseScenInfo = {
  eth_opts: {
    provider: 'ganache', // [env=PROVIDER]
    ganache: {
      opts: {},
      web3_port: null
    },
  },
  default_actor: null,
  actors: ["ashley", "bert"],
  validators: ["alice", "bob"],
  tokens: [
    "zrx",
    "dai",
    "comp",
    "bat",
    "wbtc",
    "usdc",
  ],
  chain_spec: { // TODO: Allow override of chain spec?
    base_chain: "local",
    use_temp: true,
    props: {}
  },
  starport: {},
  cash_token: {},
  contracts_dir: null, // [env=BUILD_DIR]
  log_level: 'info', // [env=LOG]
  link_validator: true, // abort if validator panics [env=LINK_VALIDATOR]
  profile: 'debug', // or debug [env=PROFILE]
  target: null, // compound-chain binary [env=CHAIN_BIN]
  types_file: null, // types.json file [env=TYPES_FILE]
};

// Helper function to take an info that might be
// either an array of objects or strings or an
// object and returns it as an entries array. Strings
// are converted to the values derived from getInfo()'s
// keys.
async function instantiateInfo(info, type, indexKey, infoMap) {
  if (Array.isArray(info)) {
    return info.map((el) => {
      if (typeof (el) === 'string') {
        if (!infoMap[el]) {
          throw new Error(`Unknown ${type}: ${el} (Available: ${Object.keys(infoMap).join(', ')})`);
        } else {
          return [el, infoMap[el]];
        }
      } else if (typeof (el) === 'object') {
        if (!el[indexKey]) {
          throw new Error(`Elements must have indexKey \`{indexKey}\` for ${type} in ${JSON.stringify(el)}`);
        }
        let index = el[indexKey];
        let baseObj = infoMap[index] ? infoMap[index] : {};
        let obj =
          {
            ...baseObj,
            ...el
          }
        delete obj[indexKey]; // Remove index key from result
        return [index, obj];
      }
    });
  } else if (typeof (info) === 'object') {
    return Object.entries(info).map(([index, obj]) => {
      return [index, {
        ...infoMap[index] ? infoMap[index] : {},
        ...obj
      }];
    });
  } else {
    throw new Error(`Invalid type for ${type}: ${JSON.stringify(info)}`);
  }
}

module.exports = {
  baseScenInfo,
  instantiateInfo
};
