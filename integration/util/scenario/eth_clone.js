const { buildCashToken } = require('./cash_token');
const { buildStarport } = require('./starport');
const { buildTokens } = require('./token');

class EthClone {
    constructor(tokens, cashToken, starport) {
        this.tokens = tokens;
        this.cashToken = cashToken;
        this.starport = starport;
    }
}


async function buildEthClone(ctx, scenInfo) {
    // Note: `4` below is the number of transactions we expect to occur between now and when
    //       the Starport token is deployed.
    //       That's now: deploy Proxy Admin (1), Cash Token Impl (2), Starport Impl (3), Proxy (4)
    let starportAddress = await ctx.eth.getNextContractAddress(4);
    const cashToken = await buildCashToken(scenInfo.cash_token, ctx, starportAddress);
    const starport = await buildStarport(scenInfo.starport, scenInfo.validators, ctx, cashToken.ethAddress());
    const tokens = await buildTokens(scenInfo.tokens, scenInfo, ctx);

    return new EthClone(tokens, cashToken, starport );
}

module.exports = {
    buildEthClone,
    EthClone
};