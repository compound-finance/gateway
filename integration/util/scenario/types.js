const Web3Utils = require('web3-utils');
const ABICoder = require('web3-eth-abi');

function getNoticeChainId(notice) {
  if (notice.Notice.ExtractionNotice && notice.Notice.ExtractionNotice.Eth) {
    return "Eth";
  } else if (notice.Notice.CashExtractionNotice && notice.Notice.CashExtractionNotice.Eth) {
    return "Eth";
  } else if (notice.Notice.FutureYieldNotice && notice.Notice.FutureYieldNotice.Eth) {
    return "Eth";
  } else if (notice.Notice.SetSupplyCapNotice && notice.Notice.SetSupplyCapNotice.Eth) {
    return "Eth";
  } else if (notice.Notice.ChangeAuthorityNotice && notice.Notice.ChangeAuthorityNotice.Eth) {
    return "Eth";
  } else {
    throw `Unknown notice chain: ${JSON.stringify(notice.Notice)}`
  }
}

function encodeNoticeWith(notice, signature, args) {
  let magic = Web3Utils.asciiToHex("ETH:");
  let [eraId, eraIndex] = notice.id;
  let parentHash = notice.parent;
  let header = ABICoder.encodeParameters(["uint256", "uint256", "uint256"], [eraId, eraIndex, parentHash]);
  let call = ABICoder.encodeFunctionCall(signature, args);

  return `${magic}${header.slice(2)}${call.slice(2)}`;
}

function encodeNotice(notice) {
  if (notice.ExtractionNotice && notice.ExtractionNotice.Eth) {
    let ethNotice = notice.ExtractionNotice.Eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: "unlock",
        type: 'function',
        inputs: [
          { name: '', type: "address" },
          { name: '', type: "uint256" },
          { name: '', type: "address" },
        ],
        outputs: []
      },
      [ethNotice.asset, ethNotice.amount, ethNotice.account]
    );
  } else if (notice.CashExtractionNotice && notice.CashExtractionNotice.Eth) {
    let ethNotice = notice.CashExtractionNotice.Eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: "unlockCash",
        type: 'function',
        inputs: [
          { name: '', type: "address" },
          { name: '', type: "uint256" },
          { name: '', type: "uint256" },
        ],
        outputs: []
      },
      [ethNotice.account, ethNotice.amount, ethNotice.cash_index]
    );
  } else if (notice.FutureYieldNotice && notice.FutureYieldNotice.Eth) {
    let ethNotice = notice.FutureYieldNotice.Eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: "setFutureYield",
        type: 'function',
        inputs: [
          { name: '', type: "uint256" },
          { name: '', type: "uint256" },
          { name: '', type: "uint256" },
        ],
        outputs: []
      },
      [ethNotice.next_cash_yield, ethNotice.next_cash_yield_start_at, ethNotice.next_cash_index]
    );
  } else if (notice.SetSupplyCapNotice && notice.SetSupplyCapNotice.Eth) {
    let ethNotice = notice.SetSupplyCapNotice.Eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: "setSupplyCap",
        type: 'function',
        inputs: [
          { name: '', type: "address" },
          { name: '', type: "uint256" },
        ],
        outputs: []
      },
      [ethNotice.asset, ethNotice.amount]
    );
  } else if (notice.ChangeAuthorityNotice && notice.ChangeAuthorityNotice.Eth) {
    let ethNotice = notice.ChangeAuthorityNotice.Eth;

    return encodeNoticeWith(
      ethNotice,
      {
        name: "changeAuthorities",
        type: 'function',
        inputs: [
          { name: '', type: "address[]" },
        ],
        outputs: []
      },
      [ethNotice.new_authorities]
    );
  } else {
    throw `Unknown notice chain: ${JSON.stringify(notice.Notice)}`
  }
}

function getNoticeParentHash(notice) {
  if (notice.ExtractionNotice && notice.ExtractionNotice.Eth) {
    return notice.ExtractionNotice.Eth.parent;
  } else if (notice.CashExtractionNotice && notice.CashExtractionNotice.Eth) {
    return notice.CashExtractionNotice.Eth.parent;
  } else if (notice.FutureYieldNotice && notice.FutureYieldNotice.Eth) {
    return notice.FutureYieldNotice.Eth.parent;
  } else if (notice.SetSupplyCapNotice && notice.SetSupplyCapNotice.Eth) {
    return notice.SetSupplyCapNotice.Eth.parent;
  } else if (notice.ChangeAuthorityNotice && notice.ChangeAuthorityNotice.Eth) {
    return notice.ChangeAuthorityNotice.Eth.parent;
  } else {
    throw `Unknown notice chain: ${JSON.stringify(notice.Notice)}`
  }
}

function getNoticeId(notice) {
  if (notice.ExtractionNotice && notice.ExtractionNotice.Eth) {
    return notice.ExtractionNotice.Eth.id;
  } else if (notice.CashExtractionNotice) {
    return notice.CashExtractionNotice.id;
  } else if (notice.FutureYieldNotice) {
    return notice.FutureYieldNotice.id;
  } else if (notice.SetSupplyCapNotice) {
    return notice.SetSupplyCapNotice.id;
  } else if (notice.ChangeAuthorityNotice) {
    return notice.ChangeAuthorityNotice.id;
  } else {
    throw `Unknown notice chain: ${JSON.stringify(notice.Notice)}`
  }
}

function getRawHash(hash) {
  if (hash.Comp) {
    return hash.Comp;
  } else if (hash.Eth) {
    return hash.Eth;
  } else if (hash.Dot) {
    return hash.Dot;
  } else if (hash.Sol) {
    return hash.Sol;
  } else if (hash.Tez) {
    return hash.Tez;
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
