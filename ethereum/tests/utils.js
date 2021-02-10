const RLP = require('rlp');
const { ethers } = require('ethers');
const { Wallet } = require("@ethersproject/wallet");

const bigInt = num => {
  return BigInt(num);
};

const e18 = num => {
  return bigInt(num) * bigInt(1e18);
};

const getNextContractAddress = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
};

const randomAddress = () =>  Wallet.createRandom().address;

const nRandomWallets = (num) => {
  return Array(num).fill(null).map(() => Wallet.createRandom());
};

const nRandomAuthorities = num => {
  const newAuths = nRandomWallets(num).map(acct => acct.address);
  const paramTypes = Array(num).fill('address');
  return ethers.utils.defaultAbiCoder.encode(paramTypes, newAuths).slice(2);
};

// No ethereum prefix, just signs the hash of the raw digest
const sign = (msg, account) => {
  const hash = ethers.utils.keccak256(msg);
  const sk = new ethers.utils.SigningKey(account.privateKey);
  const hashArr = ethers.utils.arrayify(hash);
  const sigRaw = sk.signDigest(hashArr);
  const signature = ethers.utils.joinSignature(sigRaw);

  return { hash, signature }
};

const signAll = (msg, accounts) => {
  return accounts.map(acct => sign(msg, acct).signature);
};

const replaceByte = (hexStr, index, replacement) => {
  let start = hexStr.slice(0, 2 + index * 2);
  let mid = replacement;
  let end = hexStr.slice(4 + index * 2);

  return `${start}${mid}${end}`;
}

const ETH_HEADER = "0x4554483a";
const ETH_ADDRESS = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

module.exports = {
  bigInt,
  e18,
  getNextContractAddress,
  nRandomWallets,
  nRandomAuthorities,
  replaceByte,
  sign,
  signAll,
  ETH_HEADER,
  ETH_ADDRESS
};
