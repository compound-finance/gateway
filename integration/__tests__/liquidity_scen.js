const {
  buildScenarios,
} = require('../util/scenario');

let liquidatity_scen_info = {
  tokens: [
    { token: "usdc", balances: { ashley: 1000 }, liquidity_factor: 0.9 },
    { token: "bat", balances: { bert: 25000 }, liquidity_factor: 0.3 }
  ],
};

buildScenarios('Liquidity Scenarios', liquidatity_scen_info, [
  {
    name: "Liquidity for Basic Collateral",
    scenario: async ({ ashley, usdc }) => {
      await ashley.lock(100, usdc);
      let liquidity = await ashley.liquidity();
      expect(liquidity).toEqual(90);
    }
  },
  {
    name: "Liquidity for Just Cash",
    scenario: async ({ ashley, bert, cash, usdc }) => {
      await ashley.lock(100, usdc);
      await ashley.transfer(50, cash, bert);
      expect(await bert.liquidity()).toBeCloseTo(50, 4);
    }
  },
  {
    name: "Liquidity for Collateral and Cash Borrow",
    scenario: async ({ ashley, bert, cash, usdc }) => {
      await ashley.lock(100, usdc);
      await ashley.transfer(50, cash, bert);
      expect(await ashley.liquidity()).toBeCloseTo(39.99, 4); // Why not 40?
    }
  },
  {
    name: "Liquidity for Collateral and Token Borrow",
    scenario: async ({ ashley, bert, cash, usdc, bat }) => {
      await bert.lock(100, bat);
      await ashley.lock(100, usdc); // +90
      await ashley.transfer(20, bat, bert); // -20 * 0.313242 / 0.30 = -20.882800000000003
      expect(await ashley.liquidity()).toBeCloseTo(69.1172, 1);
    }
  },
  {
    name: "Liquidity when Underwater via Liquidity Factor Change",
    scenario: async ({ api, ashley, bert, cash, usdc, starport }) => {
      await ashley.lock(100, usdc); // +90 -> +20
      await ashley.transfer(50, cash, bert); // -50
      expect(await ashley.liquidity()).toBeCloseTo(39.99, 4); // Why not 40?
      let extrinsic = api.tx.cash.setLiquidityFactor(usdc.toChainAsset(), 200000000000000000n);
      await starport.executeProposal("Reduce USDC Liquidity Factor", [extrinsic]);
      expect(await ashley.liquidity()).toBeCloseTo(-30.01, 2); // -30
    }
  }
]);
