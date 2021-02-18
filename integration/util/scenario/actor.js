const { Keyring } = require('@polkadot/api');
const { getInfoKey } = require('../util');
const { instantiateInfo } = require('./scen_info');
const { sendAndWaitForEvents } = require('../substrate');
const { lookupBy } = require('../util');
const { CashToken } = require('./cash_token');

class Actor {
  constructor(name, ethAddress, chainKey, ctx) {
    this.name = name;
    this.__ethAddress = ethAddress;
    this.chainKey = chainKey;
    this.ctx = ctx;
    this.nextId = 0;
  }

  show() {
    return this.name;
  }

  ethAddress() {
    if (!this.__ethAddress) {
      throw new Error(`Actor ${this.name} does not have a valid eth account`);
    }

    return this.__ethAddress;
  }

  toChainAccount() {
    return { Eth: this.ethAddress() };
  }

  toTrxArg() {
    return `Eth:${this.ethAddress()}`;
  }

  declareInfo() {
    return {
      active: "I am going to",
      past: "I just did",
      failed: "I failed to",
      colorId: [...this.name].reduce((acc, el) => el.charCodeAt(0) + acc, 0),
      id: this.nextId++,
      name: this.name
    };
  }

  declare(...args) {
    return this.ctx.declare(this.declareInfo(), ...args);
  }

  async nonce() {
    return await this.ctx.api().query.cash.nonces(this.toChainAccount());
  }

  async sign(data) {
    return await this.ctx.eth.sign(data, this);
  }

  async signWithNonce(data) {
    let currentNonce = await this.nonce();
    let signature = await this.sign(`${currentNonce}:${data}`);

    return [ { Eth: [ this.ethAddress(), signature ] }, currentNonce ];
  }

  async runTrxRequest(trxReq) {
    let [sig, currentNonce] = await this.signWithNonce(trxReq);
    let call = this.ctx.api().tx.cash.execTrxRequest(trxReq, sig, currentNonce);

    return await sendAndWaitForEvents(call, this.ctx.api(), false);
  }

  async ethBalance() {
    return await this.ctx.eth.ethBalance(this);
  }

  async tokenBalance(tokenLookup) {
    let token = this.ctx.tokens.get(tokenLookup);
    return await token.getBalance(this);
  }

  async chainCashPrincipal() {
    let principal = await this.ctx.api().query.cash.cashPrincipals(this.toChainAccount());
    return principal.toNumber();
  }

  async chainCashBalance() {
    return await this.chainCashPrincipal() * await this.ctx.chain.cashIndex();
  }

  async chainBalance(tokenLookup) {
    let token = this.ctx.tokens.get(tokenLookup);
    if (token instanceof CashToken) {
      return token.toTokenAmount(await this.chainCashBalance());
    } else {
      let weiAmount = await this.ctx.api().query.cash.assetBalances(token.toChainAsset(), this.toChainAccount());
      return token.toTokenAmount(weiAmount);
    }
  }

  async lock(amount, asset, awaitEvent = true) {
    return await this.declare("lock", [amount, asset], async () => {
      let lockRes = await this.ctx.starport.lock(this, amount, asset);
      if (awaitEvent) {
        await this.ctx.chain.waitForEthProcessEvent('cash', 'GoldieLocks'); // Replace with real event
      }
      return lockRes;
    });
  }

  async extract(amount, asset, recipient = null) {
    return await this.declare("extract", [amount, asset, "for", recipient || "myself"], async () => {
      let token = this.ctx.tokens.get(asset);
      let weiAmount = token.toWeiAmount(amount);

      let trxReq = this.extractTrxReq(amount, asset, recipient);

      this.ctx.log(`Running Trx Request \`${trxReq}\` from ${this.name}`);

      return await this.runTrxRequest(trxReq);
    });
  }

  async execTrxRequest(trxRequest, awaitEvent = true) {
    return await this.ctx.starport.execTrxRequest(this, trxRequest, awaitEvent);
  }

  extractTrxReq(amount, asset, recipient = null) {
    let token = this.ctx.tokens.get(asset);
    let weiAmount = token.toWeiAmount(amount);

    return this.ctx.generateTrxReq("Extract", weiAmount, token, recipient || this)
  }

  async transfer(amount, asset, recipient) {
    return await this.declare("transfer", [amount, asset, "to", recipient], async () => {
      let token = this.ctx.tokens.get(asset);
      let weiAmount = token.toWeiAmount(amount);

      let trxReq = this.ctx.generateTrxReq("Transfer", weiAmount, token, recipient);

      this.ctx.log(`Running Trx Request \`${trxReq}\` from ${this.name}`);

      return await this.runTrxRequest(trxReq);
    });
  }
}

class Actors {
  constructor(actors, keyring, ctx) {
    this.actors = actors;
    this.keyring = keyring;
    this.ctx = ctx;
  }

  all() {
    return this.actors;
  }

  get(lookup) {
    return lookupBy(Actor, 'name', this.actors, lookup);
  }
}

function actorInfoMap(keyring) {
  return {
    ashley: {
      key_uri: '//Alice'
    },
    bert: {
      key_uri: '//Bob'
    }
  };
}

async function buildActor(actorName, actorInfo, keyring, index, ctx) {
  let ethAddress = ctx.eth.accounts[index + 1];
  let chainKey = keyring.addFromUri(getInfoKey(actorInfo, 'key_uri', `actor ${actorName}`))

  return new Actor(actorName, ethAddress, chainKey, ctx);
}

async function getActorsInfo(actorsInfoHash, keyring, ctx) {
  let actorInfoMap = actorInfo(keyring);

  if (Array.isArray(actorsInfoHash)) {
    return actorsInfoHash.map((t) => {
      if (typeof (t) === 'string') {
        if (!actorInfoMap[t]) {
          throw new Error(`Unknown Actor: ${t}`);
        } else {
          return [t, actorInfoMap[t]];
        }
      } else {
        let {
          name,
          ...restActor
        } = t;
        return [name, restActor];
      }
    });
  } else {
    return Object.entries(actorsInfoHash);
  }
}

async function buildActors(actorsInfoHash, defaultActor, ctx) {
  let keyring = new Keyring();

  let actorsInfo = await instantiateInfo(actorsInfoHash, 'Actor', 'name', actorInfoMap(keyring));
  let actors = await Promise.all(actorsInfo.map(([actorName, actorInfo], index) => {
    return buildActor(actorName, actorInfo, keyring, index, ctx);
  }));

  // TODO: Default actor
  return new Actors(actors, keyring, ctx);
}

module.exports = {
  Actor,
  Actors,
  buildActor,
  buildActors
};
