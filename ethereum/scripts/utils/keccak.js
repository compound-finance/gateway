const Web3Utils = require('web3-utils');
const fs = require('fs').promises;

keccakFile = async (file) => {
  let data = await fs.readFile(file);
  return Web3Utils.keccak256(data);
}

let [_p, _s, file] = process.argv;

if (!file) {
  throw new Error(`usage: keccak.js <filename>`);
}

keccakFile(file).then((keccak) => console.log(keccak));
