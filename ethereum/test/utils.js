const RLP = require('rlp');
const { ethers } = require('ethers');
const { Wallet } = require("@ethersproject/wallet");
const Web3Utils = require('web3-utils');

const bigInt = num => {
  return BigInt(num);
};

const e18 = num => {
  return bigInt(num) * bigInt(1e18);
};

const e6 = num => {
  return bigInt(num) * bigInt(1e6);
};

const toPrincipal = (n, index = 1e18) => {
  return Number(n) * 1e18 / index;
};

const getNextContractAddress = (acct, nonce) => {
  return '0x' + Web3Utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
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

const sendRPC = (web3_, method, params) => {
  return new Promise((resolve, reject) => {
    if (!web3_.currentProvider || typeof (web3_.currentProvider) === 'string') {
      return reject(`cannot send from currentProvider=${web3_.currentProvider}`);
    }

    web3_.currentProvider.send(
      {
        jsonrpc: '2.0',
        method: method,
        params: params,
        id: new Date().getTime() // Id of the request; anything works, really
      },
      (err, response) => {
        if (err) {
          reject(err);
        } else {
          resolve(response);
        }
      }
    );
  });
}

const fromNow = (seconds) => {
  return Math.floor(seconds + (new Date() / 1000));
}

const ETH_HEADER = "0x4554483a";
const ETH_ADDRESS = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";
const ETH_ZERO_ADDRESS = "0x0000000000000000000000000000000000000000";

async function call(contract, fn, args = [], callArgs = {}) {
  return await contract[fn](...args, callArgs);
}

async function deploy(ethers, contract, args = [], deployArgs = {}) {
  let {
    from,
    ...restArgs
  } = deployArgs;
  const factory = await ethers.getContractFactory(contract, from);
  return await factory.deploy(...args, restArgs);
}

async function getContractAt(ethers, contract, address, signer) {
  return await ethers.getContractAt(contract, address, signer);
}

module.exports = {
  bigInt,
  call,
  deploy,
  e18,
  e6,
  fromNow,
  getContractAt,
  getNextContractAddress,
  nRandomWallets,
  nRandomAuthorities,
  replaceByte,
  sign,
  signAll,
  sendRPC,
  toPrincipal,
  ETH_HEADER,
  ETH_ADDRESS,
  ETH_ZERO_ADDRESS
};
