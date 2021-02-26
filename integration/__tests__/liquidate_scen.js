const {
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let liquidate_scen_info = {
  tokens: [
    { token: "usdc", balances: { ashley: 1000 }, liquidity_factor: 0.9 },
    { token: "bat", balances: { bert: 25000 }, liquidity_factor: 0.9 }
  ],
};

async function getUnhealthy({ ashley, usdc, bert, chuck, cash, ctx, starport }) {
  await ashley.lock(100, usdc); // +90 -> +20
  await ashley.transfer(50, cash, chuck); // -50
  expect(await ashley.liquidity()).toBeCloseTo(39.99, 4); // Why not 40?
  let extrinsic = ctx.api().tx.cash.setLiquidityFactor(usdc.toChainAsset(), 200000000000000000n);
  await starport.executeProposal("Reduce USDC Liquidity Factor", [extrinsic]);
  expect(await ashley.liquidity()).toBeCloseTo(-30.01, 2); // -30
}

buildScenarios('Liquidate Scenarios', liquidate_scen_info, { beforeEach: getUnhealthy }, [
  {
    name: "Liquidate Collateral",
    scenario: async ({ ashley, bert, bat, usdc, cash, log }) => {
      await bert.lock(1000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.cash()).toEqual(0);
      await bert.liquidate(20, cash, usdc, ashley);

      let ashleyLiquidityAfter = await ashley.liquidity();
      let bertLiquidityAfter = await bert.liquidity();

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(78.40, 2); // 100 - (20 * 1.08)
      expect(await ashley.cash()).toBeCloseTo(-30.01, 2); // 50 + 20
      expect(await bert.chainBalance(usdc)).toBeCloseTo(21.60, 2); // (20 * 1.08)
      expect(await bert.cash()).toBeCloseTo(-20, 2);

      expect(ashleyLiquidityBefore).toBeCloseTo(-30.01, 2);
      expect(ashleyLiquidityAfter).toBeCloseTo(-14.33, 2); // Note: liquidity increases for target
      expect(bertLiquidityBefore).toBeCloseTo(281.9178, 2);
      expect(bertLiquidityAfter).toBeCloseTo(266.24, 2); // Note: liquidity decreases for liquidator
    }
  }
]);
