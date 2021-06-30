
const { buildCashToken } = require('./cash_token');
const { buildStarport } = require('./starport');

class Deployment {
    constructor(starport, cashToken, chain, ctx) {
        this.starport = starport;
        this.cashToken = cashToken;
        this.chain = chain;
        this.name = `${chain.name}Deployment`
        this.ctx = ctx;
    }
}

class Deployments {
    constructor(deployments, ctx) {
        this.deployments = deployments;
        this.ctx = ctx;
    }

    all() {
        return this.deployments;
    }

    starports() {
        return this.deployments.map(deployment => deployment.starport)
    }

    cashTokens() {
        return this.deployments.map(deployment => deployment.cashToken)
    }

    starportsForChainSpec() {
        return this.starports().map(starport => starport.chainAddressStr());
    }
}

async function buildDeployment(scenInfo, chain, ctx) {
    // Note: `3` below is the number of transactions we expect to occur between now and when
    //       the Starport token is deployed.
    //       That's now: deploy Proxy Admin (1), Cash Token Impl (2), Starport Impl (3), Proxy (4)
    let starportAddress = await chain.getNextContractAddress(4);
    const cashToken = await buildCashToken(scenInfo.cash_token, ctx, starportAddress, chain);
    const starport = await buildStarport(scenInfo.starport, scenInfo.validators, ctx, chain, cashToken);

    return new Deployment(starport, cashToken, chain, ctx);
}

async function buildDeployments(scenInfo, ctx) {
    const deployments = await Promise.all(
        ctx.chains.all()
        .map(chain => buildDeployment(scenInfo, chain, ctx))
    );


    return new Deployments(deployments, ctx);
}
module.exports = {
    buildDeployments
};
