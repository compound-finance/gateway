const { buildScenarios } = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let now = Date.now();

let lock_scen_info = {
    tokens: [
        { token: 'usdc', balances: { ashley: 1000 } },
        { token: 'maticZrx', balances: { darlene: 1000000 } },
        { token: 'zrx', balances: { bert: 1000000 } },
        { token: 'comp' }
    ],
    validators: ['alice', 'bob'],
    actors: ['ashley', 'bert', 'chuck', 'darlene', 'edward'],
    chain_opts: {
        matic: {
            name: 'matic',
            provider: 'ganache',
            ganache: {
                opts: {},
                web3_port: null
            },
        },
    },
    native: true, // for log messages - removes wasm:stripped - no need for wasm no forkless upgrade testing here
    freeze_time: now,
    initial_yield: 300,
    initial_yield_start_ms: now
};

buildScenarios('Matic', lock_scen_info, [
    {
        name: 'Matic Lock',
        scenario: async ({ashley, usdc, darlene, maticZrx }) => {
            // Lock on Polygon
            let balance, liquidity;
            await darlene.lock(100, maticZrx);
            balance = await darlene.balanceForToken(maticZrx);
            let darleneLiquidity = await darlene.liquidity();
            expect(balance).toEqual(100);
            expect(darleneLiquidity).toBeGreaterThan(0);

            // lock on ethereum
            await ashley.lock(1000, usdc);
            balance = await ashley.balanceForToken(usdc);
            liquidity = await ashley.liquidity();
            expect(balance).toEqual(1000);
            expect(liquidity).toBeGreaterThan(0);

            // lock on polygon again
            await darlene.lock(100, maticZrx);
            balance = await darlene.balanceForToken(maticZrx);
            liquidity = await darlene.liquidity();
            expect(balance).toEqual(200); // double the asset balance (no interest, 0 utilization)
            expect(liquidity).toEqual(2*darleneLiquidity); // liquidity must have gone up
        }
    },
    {
        name: 'Collateral Borrowed Interest Lump Sum',
        scenario: async ({ darlene, chuck, cash, chain, usdc, maticZrx}) => {
            // await prices.postPrices();
            let balance, liquidity;
            await chain.setFixedRate(usdc, 500); // 5% APY fixed
            await darlene.lock(10000, maticZrx);
            balance = await darlene.balanceForToken(maticZrx);
            let liquidity1 = await darlene.liquidity();
            expect(balance).toEqual(10000);
            expect(liquidity1).toBeGreaterThan(0);
            // now we know everything is in order, make the transfer, let's say half our liquidity worth
            await darlene.transfer(1000, usdc, chuck);
            expect(await darlene.chainBalance(usdc)).toEqual(-1000);
            expect(await chuck.chainBalance(usdc)).toEqual(1000);
            expect(await darlene.chainBalance(cash)).toBeCloseTo(-0.01, 2); // 1¢ transfer fee
            expect(await chuck.chainBalance(cash)).toEqual(0);
            await chain.accelerateTime({years: 1});
            expect(await darlene.chainBalance(usdc)).toEqual(-1000);
            expect(await chuck.chainBalance(usdc)).toEqual(1000);
            expect(await darlene.chainBalance(cash)).toBeCloseTo(-51.53272669767585, 3); // -50 * Math.exp(0.03) - 0.01
            expect(await chuck.chainBalance(cash)).toBeCloseTo(51.52272669767585, 3); // 50 * Math.exp(0.03)
        }
    },
    {
        name: "Extract Collateral",
        scenario: async ({ darlene, maticZrx, chain, maticStarport }) => {
            let assetInfo = await maticZrx.getAssetInfo();

            let balance, liquidity;
            // start with 1,000,000
            // lock 10,000
            // on matic = 990,000
            // on gateway = 10,000
            await darlene.lock(10000, maticZrx);
            balance = await darlene.balanceForToken(maticZrx);
            expect(balance).toEqual(10000); // gateway balance not yet debited
            expect(await darlene.tokenBalance(maticZrx)).toEqual(990000); // matic balance not yet credited
            liquidity = await darlene.liquidity();
            expect(liquidity).toBeGreaterThan(0);

            // request extract 50 maticZrx
            let extract = await darlene.extract(50, maticZrx);
            let notice = getNotice(extract);
            let signatures = await chain.getNoticeSignatures(notice);

            // before we submit - query the balances to ensure they are as expected
            balance = await darlene.balanceForToken(maticZrx);
            expect(balance).toEqual(9950); // gateway balance debited
            expect(await darlene.tokenBalance(maticZrx)).toEqual(990000); // matic balance not yet credited
            // now invoke the notice!
            await maticStarport.invoke(notice, signatures);
            // balances are updated everywhere, should now be
            // on matic = 990,050
            // on gateway = 9,950
            balance = await darlene.balanceForToken(maticZrx);
            expect(balance).toEqual(9950); // gateway balance debited
            expect(await darlene.tokenBalance(maticZrx)).toEqual(990050); // matic balance now credited
        }
    },
]);
