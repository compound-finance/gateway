const { getValidatorsInfo } = require('./validator');
const { EtherToken } = require('./token');
const { encodeCall } = require('../substrate');
const web3 = require('web3');

class Starport {
  constructor(starport, proxyAdmin, starportImpl, proxy, starportTopics, cashToken, ctx, chain) {
    this.starport = starport;
    this.proxyAdmin = proxyAdmin;
    this.starportImpl = starportImpl;
    this.proxy = proxy;
    this.starportTopics = starportTopics;
    this.cashToken = cashToken;
    this.ctx = ctx;
    this.chain = chain;
    this.ctxKey = `${chain.name}Starport`
  }

  ethAddress() {
    return this.starport._address;
  }

  chainAddressStr() {
    return `${this.chain.name.toUpperCase()}:${this.ethAddress()}`;
  }

  chainAddress() {
    // xxx todo:wn parameterize by chain
    return { Eth: this.ethAddress() };
  }

  topics() {
    return this.starportTopics;
  }

  async approve(actorLookup, amount, collateral) {
    let actor = this.ctx.actors.get(actorLookup);
    let token = this.ctx.tokens.get(collateral);

    await token.approve(actor, this.ethAddress(), amount);
  }

  async lockEth(actorLookup, weiAmount) {
    let actor = this.ctx.actors.get(actorLookup);
    // Note: we use gas price = 0 for tests to prevent this from scewing the eth balance of the user
    return await this.starport.methods.lockEth().send({ value: weiAmount, from: actor.ethAddress(), gasPrice: "0" });
  }

  async lock(actorLookup, amount, collateral, approve=true) {
    let actor = this.ctx.actors.get(actorLookup);
    let token = this.ctx.tokens.get(collateral);
    let weiAmount = token.toWeiAmount(amount);

    if (token instanceof EtherToken) {
      return await this.lockEth(actor, weiAmount);
    } else {
      if (approve) {
        await this.approve(actor, amount, collateral);
      }
      return await this.starport.methods.lock(weiAmount, token.ethAddress()).send({ from: actor.ethAddress() });
    }
  }

  // TODO: Add `lockTo`

  async setSupplyCap(token, amount) {
    let weiAmount = token.toWeiAmount(amount);

    return await this.starport.methods.setSupplyCap(token.ethAddress(), weiAmount).send({ from: this.chain.root() });
  }

  async executeProposal(title, extrinsics, opts = {}) {
    opts = {
      awaitEvent: true,
      awaitNotice: false,
      checkSuccess: true,
      ethOpts: {},
      ...opts,
    };
    let encodedCalls = extrinsics.map(encodeCall);
    let result = await this.starport.methods.executeProposal(title, encodedCalls).send({ from: this.ctx.eth.root(), ...opts.ethOpts });
    let event;
    let notice;
    if (opts.awaitNotice) {
      notice = await this.ctx.chain.waitForNotice();
    }
    if (opts.awaitEvent) {
      event = await this.ctx.chain.waitForEthProcessEvent('cash', 'ExecutedGovernance');

      if (opts.checkSuccess) {
        let [payload, govResult] = event.data[0][0];
        if (!govResult.isDispatchSuccess) {
          expect(govResult.toJSON()).toBe(null);
        }
      }
    }

    return {
      event,
      notice,
      result
    };
  }

  async isNoticeInvoked(noticeHash) {
    return await this.starport.methods.isNoticeInvoked(noticeHash).call();
  }

  async getAuthorities() {
    return (await this.starport.methods.getAuthorities().call()).map(web3.utils.toChecksumAddress);
  }

  async execTrxRequest(actorLookup, trxReq, awaitEvent = true) {
    let actor = this.ctx.actors.get(actorLookup);

    let event;
    let tx = this.starport.methods.execTrxRequest(trxReq).send({ from: actor.ethAddress() });
    if (awaitEvent) {
      // TODO: Pass in log id?
      event = await this.ctx.chain.waitForChainProcessed();
    }
    return {
      tx,
      event
    };
  }

  async invoke(notice, signaturePairs) {
    let encoded = notice.EncodedNotice;
    let signatures = signaturePairs.map(([_, sig]) => sig);
    let sendOpts = { from: this.chain.defaultFrom, gas: 5000000 };
    return await this.starport.methods.invoke(encoded, signatures).send(sendOpts);
  }

  async invokeChain(target, notices) {
    let encodedTarget = target.EncodedNotice;
    let encodedNotices = notices.map((n) => typeof(n) === 'string' ? n : n.EncodedNotice);
    return await this.starport.methods.invokeChain(encodedTarget, encodedNotices).send({ from: this.ctx.eth.defaultFrom, gas: 5000000 });
  }

  async upgradeTo(version) {
    let newImpl = await this.ctx.eth.__deploy('Starport', [this.cashToken.ethAddress(), this.ctx.eth.root()], { version });
    await this.upgrade(newImpl);
  }

  async upgrade(impl, upgradeCall = null) {
    if (upgradeCall) {
      await this.proxyAdmin.methods.upgradeAndCall(
        this.starport._address,
        impl._address,
        upgradeCall
      ).send({ from: this.ctx.eth.root() });
    } else {
      let tx = await this.proxyAdmin.methods.upgrade(this.starport._address, impl._address).send({ from: this.ctx.eth.root() });
    }

    this.starport = this.ctx.eth.__getContractAtAbi(impl._jsonInterface, this.proxy._address);
  }

  async supplyCap(tokenLookup) {
    let token = this.ctx.tokens.get(tokenLookup);

    return this.starport.methods.supplyCaps(token.ethAddress()).call();
  }

  async tokenBalance(tokenLookup) {
    let token = this.ctx.tokens.get(tokenLookup);
    return await token.getBalance(this.starport._address);
  }
}

async function buildStarport(starportInfo, validatorsInfoHash, ctx, chain, cashToken) {
  ctx.log(`Deploying Starport to ${chain.name}...`);

  if (!cashToken) {
    throw new Error(`Cannot deploy Starport without first deploying Cash Token`);
  }
  let validatorsInfo = await getValidatorsInfo(validatorsInfoHash, ctx);
  let validators = validatorsInfo.map(([_, v]) => v.eth_account);

  if (starportInfo.existing) {
    // Use existing Starport information
    ['proxy_admin', 'proxy', 'starport', 'starport_impl'].forEach((key) => {
      if (!starportInfo.existing[key]) {
        throw new Error(`Existing Starport missing property: ${key}`);
      }
    });

    let proxyAdmin = starportInfo.existing.proxy_admin;
    let proxy = await chain.__getContractAt('TransparentUpgradeableProxy', starportInfo.existing.proxy);
    let starport = await chain.__getContractAt('Starport', starportInfo.existing.starport);
    let starportImpl = await chain.__getContractAt('Starport', starportInfo.existing.starport_impl);
    // TODO: Allow versioning?
    let starportTopics = Object.fromEntries(starport
      ._jsonInterface
      .filter(e => e.type === 'event')
      .map(e => [e.name, e.signature]));

    return new Starport(starport, proxyAdmin, starportImpl, proxy, starportTopics, cashToken, ctx, chain);
  } else {
    // Deploy Proxies and Starport
    let proxyAdmin = cashToken.proxyAdmin;
    let starportImpl = await chain.__deploy('Starport', [cashToken.ethAddress(), chain.root(), web3.utils.asciiToHex(chain.nameAsUpperCase()), web3.utils.asciiToHex(chain.nameAsStarportHeader())]);
    let proxy = await chain.__deploy('TransparentUpgradeableProxy', [
      starportImpl._address,
      proxyAdmin._address,
      "0x"
    ], {from: chain.root()});
    let starport = await chain.__getContractAt('Starport', proxy._address);
    if (validators.length > 0) {
      await starport.methods.changeAuthorities(validators).send({from: chain.root(), gas: 400_000});
    }

    let starportTopics = Object.fromEntries(starport
      ._jsonInterface
      .filter(e => e.type === 'event')
      .map(e => [e.name, e.signature])
    );

    return new Starport(starport, proxyAdmin, starportImpl, proxy, starportTopics, cashToken, ctx, chain);
  }
}

module.exports = {
  buildStarport,
  Starport
};
