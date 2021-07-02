const Web3Utils = require('web3-utils');
const ABICoder = require('web3-eth-abi');

function getNoticeChainId(notice) {
  if (notice.Notice.extractionNotice && notice.Notice.extractionNotice.eth) {
    return 'Eth';
  } else if (notice.Notice.cashExtractionNotice && notice.Notice.cashExtractionNotice.eth) {
    return 'Eth';
  } else if (notice.Notice.futureYieldNotice && notice.Notice.futureYieldNotice.eth) {
    return 'Eth';
  } else if (notice.Notice.setSupplyCapNotice && notice.Notice.setSupplyCapNotice.eth) {
    return 'Eth';
  } else if (notice.Notice.changeAuthorityNotice && notice.Notice.changeAuthorityNotice.eth) {
    return 'Eth';
  } else if (notice.Notice.extractionNotice && notice.Notice.extractionNotice.matic) {
    return 'Matic';
  } else if (notice.Notice.cashExtractionNotice && notice.Notice.cashExtractionNotice.matic) {
    return 'Matic';
  } else if (notice.Notice.futureYieldNotice && notice.Notice.futureYieldNotice.matic) {
    return 'Matic';
  } else if (notice.Notice.setSupplyCapNotice && notice.Notice.setSupplyCapNotice.matic) {
    return 'Matic';
  } else if (notice.Notice.changeAuthorityNotice && notice.Notice.changeAuthorityNotice.matic) {
    return 'Matic';
  } else {
    throw `Unknown notice chain in getNoticeChainId: ${JSON.stringify(notice.Notice)}`;
  }
}

function encodeNoticeWith(notice, signature, args) {
  let magic = Web3Utils.asciiToHex('ETH:');
  let [eraId, eraIndex] = notice.id;
  let parentHash = notice.parent;
  let header = ABICoder.encodeParameters(['uint256', 'uint256', 'uint256'], [eraId, eraIndex, parentHash]);
  let call = ABICoder.encodeFunctionCall(signature, args);

  return `${magic}${header.slice(2)}${call.slice(2)}`;
}

function encodeNotice(notice) {
  if (notice.extractionNotice && notice.extractionNotice.eth) {
    let ethNotice = notice.extractionNotice.eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: 'unlock',
        type: 'function',
        inputs: [
          { name: '', type: 'address' },
          { name: '', type: 'uint256' },
          { name: '', type: 'address' },
        ],
        outputs: [],
      },
      [ethNotice.asset, ethNotice.amount, ethNotice.account]
    );
  } else if (notice.cashExtractionNotice && notice.cashExtractionNotice.eth) {
    let ethNotice = notice.cashExtractionNotice.eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: 'unlockCash',
        type: 'function',
        inputs: [
          { name: '', type: 'address' },
          { name: '', type: 'uint128' },
        ],
        outputs: [],
      },
      [ethNotice.account, ethNotice.principal]
    );
  } else if (notice.futureYieldNotice && notice.futureYieldNotice.eth) {
    let ethNotice = notice.futureYieldNotice.eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: 'setFutureYield',
        type: 'function',
        inputs: [
          { name: '', type: 'uint256' },
          { name: '', type: 'uint256' },
          { name: '', type: 'uint256' },
        ],
        outputs: [],
      },
      [ethNotice.next_cash_yield, ethNotice.next_cash_yield_start_at, ethNotice.next_cash_index]
    );
  } else if (notice.setSupplyCapNotice && notice.setSupplyCapNotice.eth) {
    let ethNotice = notice.setSupplyCapNotice.eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: 'setSupplyCap',
        type: 'function',
        inputs: [
          { name: '', type: 'address' },
          { name: '', type: 'uint256' },
        ],
        outputs: [],
      },
      [ethNotice.asset, ethNotice.amount]
    );
  } else if (notice.changeAuthorityNotice && notice.changeAuthorityNotice.eth) {
    let ethNotice = notice.changeAuthorityNotice.eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: 'changeAuthorities',
        type: 'function',
        inputs: [{ name: '', type: 'address[]' }],
        outputs: [],
      },
      [ethNotice.new_authorities]
    );
  } else {
    throw `Unknown notice chain in encodeNotice: ${JSON.stringify(notice.Notice)}`;
  }
}

function getNoticeParentHash(notice) {
  if (notice.extractionNotice && notice.extractionNotice.eth) {
    return notice.extractionNotice.eth.parent;
  } else if (notice.cashExtractionNotice && notice.cashExtractionNotice.eth) {
    return notice.cashExtractionNotice.eth.parent;
  } else if (notice.futureYieldNotice && notice.futureYieldNotice.eth) {
    return notice.futureYieldNotice.eth.parent;
  } else if (notice.setSupplyCapNotice && notice.setSupplyCapNotice.eth) {
    return notice.setSupplyCapNotice.eth.parent;
  } else if (notice.changeAuthorityNotice && notice.changeAuthorityNotice.eth) {
    return notice.changeAuthorityNotice.eth.parent;
  } else {
    throw `Unknown notice chain in getNoticeParentHash: ${JSON.stringify(notice.Notice)}`;
  }
}

function getNoticeId(notice) {
  if (notice.extractionNotice && notice.extractionNotice.eth) {
    return notice.extractionNotice.eth.id;
  } else if (notice.cashExtractionNotice) {
    return notice.cashExtractionNotice.id;
  } else if (notice.futureYieldNotice) {
    return notice.futureYieldNotice.id;
  } else if (notice.setSupplyCapNotice) {
    return notice.setSupplyCapNotice.id;
  } else if (notice.changeAuthorityNotice) {
    return notice.changeAuthorityNotice.id;
  } else {
    throw `Unknown notice chain in getNoticeId: ${JSON.stringify(notice.Notice)}`;
  }
}

function getRawHash(hash) {
  if (hash.comp) {
    return hash.comp;
  } else if (hash.eth) {
    return hash.eth;
  } else if (hash.dot) {
    return hash.dot;
  } else if (hash.sol) {
    return hash.sol;
  } else if (hash.tez) {
    return hash.tez;
  } else {
    throw new Error(`Unknown hash: ${JSON.stringify(hash)}`);
  }
}

module.exports = {
  getNoticeChainId,
  encodeNotice,
  getNoticeParentHash,
  getNoticeId,
  getRawHash,
};
