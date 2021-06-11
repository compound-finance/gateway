const { buildScenarios, } = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let liquidate_scen_info = {
  tokens: [
    { token: "usdc", balances: { ashley: 1000, chuck: 1000 }, liquidity_factor: 0.9 },
    { token: "bat", balances: { bert: 25000 }, liquidity_factor: 0.9 },
    { token: "comp", balances: { ashley: 100 }, liquidity_factor: 0.5 },
    { token: "zrx", balances: { ashley: 100 }, liquidity_factor: 0.5 }
  ],
  prices: {
    prices: {
      "BAT": {
        price: "0.313242",
        payload: "0x00000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000060124a7000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000004c79a0000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034241540000000000000000000000000000000000000000000000000000000000",
        signature: "0x48eaea4f4a52601aa0010bb37780989c06402f244959770d494ef040702cdbb0594b74185d36e4f09b36ca5ee270768d3252091060c53798cc479d16b3ec40e0000000000000000000000000000000000000000000000000000000000000001c",
      },
      "COMP": {
        price: "229.125",
        payload: "0x00000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000060124a7000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000da82b88000000000000000000000000000000000000000000000000000000000000000670726963657300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004434f4d5000000000000000000000000000000000000000000000000000000000",
        signature: "0x8e248059830bc2affb38f656f576e1b513e23fc9d30fb3c0193427f4b94524637b7f84eba4619116860b1cdd3987c447e0a3ae97c0adc614592b9f4dd9de14a3000000000000000000000000000000000000000000000000000000000000001b",
      },
      "ETH": {
        price: "1277.15",
        payload: "0x00000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000060124aac00000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000004c1fc3300000000000000000000000000000000000000000000000000000000000000006707269636573000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
        signature: "0xa568da2015060b2292b9587771beae4d89968f00f613bf4800dc70addacf8fddcb6d0e9550018869ba5cf1da8665daf8e0f366a0af2ae38362ea006ec260fd4a000000000000000000000000000000000000000000000000000000000000001c",
      }
    }
  }
};

let newPriceCOMP = {
  price: "472.07",
  payload: "0x00000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000060775f5000000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000001c233770000000000000000000000000000000000000000000000000000000000000000670726963657300000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004434f4d5000000000000000000000000000000000000000000000000000000000",
  signature: "0xb4442c9b1f304220a025df0ce1d6c36a7161023780132963ea2f6aa67c2dc2d901ec2c343627829f77e7c1acd545930c322a023ff20f0bdd1a587511c49e04e7000000000000000000000000000000000000000000000000000000000000001b"
};

async function supplyUSDC_BorrowCash_ChangeLF({ api, ashley, usdc, bert, chuck, cash, starport }) {
  await ashley.lock(100, usdc); // [Ash Liq. = +90]
  await ashley.transfer(49.99, cash, chuck); // // [Ash Liq. = +90 -49.99 -1¢ = +40]
  expect(await ashley.liquidity()).toBeCloseTo(40, 4);
  let extrinsic = api.tx.cash.setLiquidityFactor(usdc.toChainAsset(), 200000000000000000n);
  await starport.executeProposal("Reduce USDC Liquidity Factor", [extrinsic]);
  expect(await ashley.liquidity()).toBeCloseTo(-30, 4); // [Ash Liq. = +20 -49.99 -1¢ = -30]
}

async function receiveCash_BorrowUSDC_ChangeLF({ api, ashley, usdc, bert, chuck, cash, starport }) {
  await chuck.lock(1000, usdc);
  await chuck.transfer(100.01, cash, ashley); // [Ash Liq. = +100.01]
  await ashley.transfer(54, usdc, chuck); // // [Ash Liq. = +100.01 - 1¢ - [54 / 90%] = +40]
  expect(await ashley.liquidity()).toBeCloseTo(40, 4);
  let extrinsic = api.tx.cash.setLiquidityFactor(usdc.toChainAsset(), 200000000000000000n);
  await starport.executeProposal("Reduce USDC Liquidity Factor", [extrinsic]);
  expect(await ashley.liquidity()).toBeCloseTo(-170, 4); // [Ash Liq. = +100.01 - 1¢ - [54 / 20%] = -170]
}

async function supplyUSDC_BorrowCOMP({ api, ashley, comp, usdc, bert, chuck, cash, starport, chain }) {
  await ashley.lock(600, usdc); // [Ash Liq. = -0.01¢ [+500 * 90%] = +539.99]
  await ashley.transfer(1, comp, chuck); // [Ash Liq. = +539.99 - [229.125 / 50%] = +81.74]
  expect(await ashley.liquidity()).toBeCloseTo(81.74, 4);
}

async function supplyUSDC_BorrowCOMP_ChangePrice(ctx) {
  let { api, ashley, comp, usdc, bert, chuck, cash, starport, chain } = ctx;
  await supplyUSDC_BorrowCOMP(ctx);
  await chain.postPrice(newPriceCOMP.payload, newPriceCOMP.signature);
  expect(await ashley.liquidity()).toBeCloseTo(-404.15, 2); // [Ash Liq. = +539.99 - [472.07 / 50%] = -404.13]
}

buildScenarios('Liquidate Scenarios', liquidate_scen_info, [
  {
    name: "Liquidate Cash Borrow, Seizing USDC",
    notes:
      `Ashley supplied USDC, borrowed CASH and then we dropped the USDC
       liquidity factor from 90% to 20%, putting her underwater. Bert
       meanwhile has supplied BAT and is willing to liquidate Ashley. He will
       assume her CASH debt and receive some of her USDC.`,
    before: supplyUSDC_BorrowCash_ChangeLF,
    scenario: async ({ ashley, bert, bat, usdc, cash, log }) => {
      await bert.lock(1000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(cash)).toEqual(0);

      // Liquidate 20 CASH. Bert should receive [20 / 1.00 * 1.08]=21.60 USDC
      await bert.liquidate(20, cash, usdc, ashley);

      let ashleyLiquidityAfter = await ashley.liquidity();
      let bertLiquidityAfter = await bert.liquidity();

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(78.40, 2); // 100 Supplied - (20 * 1.08) Liquidated
      expect(await ashley.chainBalance(cash)).toBeCloseTo(-30, 2); // -50 Borrowed + 20 Cash De-Liquidated
      expect(await bert.chainBalance(usdc)).toBeCloseTo(21.60, 2); // 20 * 1.08 Seized in Liquidation
      expect(await bert.chainBalance(cash)).toBeCloseTo(-20, 2); // -20 Cash Debt Assumed in Liquidation

      expect(ashleyLiquidityBefore).toBeCloseTo(-30, 2);
      expect(ashleyLiquidityAfter).toBeCloseTo(-14.32, 2); // Note: Liquidity increased for Alice
      expect(bertLiquidityBefore).toBeCloseTo(281.9178, 2);
      expect(bertLiquidityAfter).toBeCloseTo(266.24, 2); // Note: Liquidity decreased for Bert
    }
  },
  {
    name: "Liquidate USDC Borrow, Seizing Cash",
    notes:
      `Ashley received CASH and then borrowed USDC and then we dropped the USDC
       liquidity factor from 90% to 20%, putting her underwater. Bert
       meanwhile has supplied BAT and is willing to liquidate Ashley. He will
       assume her USDC debt and receive some of her CASH.`,
    before: receiveCash_BorrowUSDC_ChangeLF,
    scenario: async ({ ashley, bert, bat, usdc, cash, log }) => {
      await bert.lock(1000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(cash)).toEqual(0);

      // Liquidate 20 USDC. Bert should receive [20 / 1.00 * 1.08]=21.60 CASH
      await bert.liquidate(20, usdc, cash, ashley);

      let ashleyLiquidityAfter = await ashley.liquidity();
      let bertLiquidityAfter = await bert.liquidity();

      expect(await ashley.chainBalance(cash)).toBeCloseTo(78.40, 2); // 100 Received - (20 * 1.08) Liquidated
      expect(await ashley.chainBalance(usdc)).toBeCloseTo(-34, 2); // -54 Borrowed + 20 USDC De-Liquidated
      expect(await bert.chainBalance(cash)).toBeCloseTo(21.60, 2); // 20 * 1.08 Seized in Liquidation
      expect(await bert.chainBalance(usdc)).toBeCloseTo(-20, 2); // -20 USDC Debt Assumed in Liquidation

      expect(ashleyLiquidityBefore).toBeCloseTo(-170, 2); // [Ash Liq. = +100.01 - 1¢ - [54 / 20%] = -170]
      expect(ashleyLiquidityAfter).toBeCloseTo(-91.60, 2); // [Ash Liq. = +78.40 - [34 / 20%]]
      expect(bertLiquidityBefore).toBeCloseTo(281.9178, 2);
      expect(bertLiquidityAfter).toBeCloseTo(203.5178, 2); // [+281.9178 + 21.60 - [20 / 0.2]]
    }
  },
  {
    name: "Liquidate COMP Borrow, Seizing USDC",
    notes:
      `Ashley locked USDC and then borrowed COMP and then COMP
       bumped up in price, putting her underwater. Bert
       meanwhile has supplied BAT and is willing to liquidate Ashley. He will
       assume her COMP debt and receive some of her USDC.`,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(1000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(comp)).toEqual(0);

      // Liquidate 0.5 COMP. Bert should receive [0.5 * 472.07 / 1.00 * 1.08]=254.9178 USDC
      await bert.liquidate(0.5, comp, usdc, ashley);

      let ashleyLiquidityAfter = await ashley.liquidity();
      let bertLiquidityAfter = await bert.liquidity();

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(345.0822, 2); // 600 Received - 254.9178 Liquidated
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-0.5, 2); // -1 Borrowed + 0.5 COMP De-Liquidated
      expect(await bert.chainBalance(usdc)).toBeCloseTo(254.9178, 2); // 254.9178 Seized in Liquidation
      expect(await bert.chainBalance(comp)).toBeCloseTo(-0.5, 2); // -0.5 COMP Debt Assumed in Liquidation

      expect(ashleyLiquidityBefore).toBeCloseTo(-404.15, 2); // [Ash Liq. = +539.99 - [472.07 / 50%] = -404.13]
      expect(ashleyLiquidityAfter).toBeCloseTo(-161.50602, 2); // [Ash Liq. = +[345.0822 • 90%] - 1¢ - [-0.5 • 472.07 / 50%] = 310.57398 - 0.01 - 472.07 = -161.50602]

      expect(bertLiquidityBefore).toBeCloseTo(281.9178, 2);
      expect(bertLiquidityAfter).toBeCloseTo(39.27382, 2); // [+281.9178 +[254.9178 • 90%] -[-0.5 • 472.07 / 50%] = 281.9178 + 229.42602 - 472.07 = 39.27382]
    }
  },
  {
    name: "Fails when liquidator has insufficient liquidity post-liquidate",
    notes:
      `A simple replay of the collateral-for-collateral scenario, but Bert has just
       a little less BAT and fails to liquidate since it would push him underwater.`,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(850, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(0.5, comp, usdc, ashley)).rejects.toThrow(/insufficientLiquidity/);

      let ashleyLiquidityAfter = await ashley.liquidity();
      let bertLiquidityAfter = await bert.liquidity();

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600, 2);
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1, 2);
      expect(await bert.chainBalance(usdc)).toBeCloseTo(0, 2);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0, 2);

      expect(ashleyLiquidityBefore).toBeCloseTo(-404.15, 2);
      expect(ashleyLiquidityAfter).toBeCloseTo(-404.15, 2);

      expect(bertLiquidityBefore).toBeCloseTo(239.63013, 2);
      expect(bertLiquidityAfter).toBeCloseTo(239.63013, 2);
    }
  },
  {
    name: "Fails when liquidator tries to close more than total borrow",
    notes:
      `A simple replay of the collateral-for-collateral scenario, but Bert goes
       for the gusto and tries to liquidate more COMP than Ashley had even borrowed.`,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(4000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(2.0, comp, usdc, ashley)).rejects.toThrow(/repayTooMuch/);

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600); // 600 Received
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1, 2); // -1 Borrowed
      expect(await bert.chainBalance(usdc)).toBeCloseTo(0, 2);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0, 2);
    }
  },
  {
    name: "Fails when liquidator tries to liquidate healthy account",
    notes:
      `A simple replay of the collateral-for-collateral scenario, but Ashley's
       account is in good standing.`,
    before: supplyUSDC_BorrowCOMP,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(4000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(0.5, comp, usdc, ashley)).rejects.toThrow(/sufficientLiquidity/);

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600); // 600 Received
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1, 2); // -1 Borrowed
      expect(await bert.chainBalance(usdc)).toBeCloseTo(0, 2);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0, 2);
    }
  },
  {
    name: "Fails when liquidator liquidates too much of borrower's given collateral",
    notes:
      `A simple replay of the collateral-for-collateral liquidation scenario, but Ashley
       doesn't have much of the seized collateral (e.g. she's mostly in USDC, not ETH).

       Note: This test is a counter-factual and should *not* pass`,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, ether, bert, bat, usdc, cash, comp, log }) => {
      await ashley.lock(0.01, ether);
      await bert.lock(4000, bat);
      let ashleyLiquidityBefore = await ashley.liquidity();
      let bertLiquidityBefore = await bert.liquidity();
      expect(await bert.chainBalance(usdc)).toEqual(0);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(0.5, comp, ether, ashley)).rejects.toThrow(/insufficientCollateral/);

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600); // 600 Received
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1, 2); // -1 Borrowed
      expect(await bert.chainBalance(usdc)).toBeCloseTo(0, 2);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0, 2);
    }
  },
  {
    name: "Fails when liquidator self-liquidates",
    notes:
      `A simple replay of the collateral-for-collateral liquidation scenario, but Ashley
       tries to liquidate herself. This is prohibited by rule.

       Note: we may need to adjust this in some way to test it after making changes
             above since if Ashley has negative liquidity to be liquidated, then clearly
             she can't have positive liquidity to also *liquidate*. It may end up impossible
             to clearly test this.
    `,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await expect(ashley.liquidate(0.5, comp, usdc, ashley)).rejects.toThrow(/selfTransfer/);

      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600);
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1);
    }
  },
  {
    name: "Fails when liquidator liquidates in-kind",
    notes:
      `A simple replay of the collateral-for-collateral liquidation scenario, but Bert
       tries to liquidate COMP for COMP. This is prohibited by rule.
    `,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(4000, bat);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(0.5, comp, comp, ashley)).rejects.toThrow(/inKindLiquidation/);

      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1); // -1 Borrowed
      expect(await bert.chainBalance(comp)).toBeCloseTo(0); // -0.5 COMP Debt Assumed in Liquidation
    }
  },
  {
    name: "Fails when liquidator liquidates below min trx value",
    notes:
      `A simple replay of the collateral-for-collateral liquidation scenario, but Bert
       tries to liquidate a very small amount of COMP that's below the min transaction threshold.
    `,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, log }) => {
      await bert.lock(4000, bat);
      expect(await bert.chainBalance(comp)).toEqual(0);

      await expect(bert.liquidate(0.0005, comp, usdc, ashley)).rejects.toThrow(/minTxValueNotMet/);

      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1);
      expect(await ashley.chainBalance(usdc)).toBeCloseTo(600);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0);
      expect(await bert.chainBalance(usdc)).toBeCloseTo(0);
    }
  },
  {
    name: "Fails when seized asset is unpriced",
    notes:
      `A simple replay of the collateral-for-collateral liquidation scenario, but we
      seize an asset that Ashley has supplied but is unpriced.`,
    before: supplyUSDC_BorrowCOMP_ChangePrice,
    scenario: async ({ ashley, bert, bat, usdc, cash, comp, zrx, log }) => {
      await ashley.lock(1, zrx);
      await bert.lock(4000, bat);
      expect(await bert.chainBalance(comp)).toEqual(0);
      await expect(bert.liquidate(0.5, comp, zrx, ashley)).rejects.toThrow(/noPrice/);
      expect(await ashley.chainBalance(comp)).toBeCloseTo(-1);
      expect(await ashley.chainBalance(zrx)).toBeCloseTo(1);
      expect(await bert.chainBalance(comp)).toBeCloseTo(0);
      expect(await bert.chainBalance(zrx)).toBeCloseTo(0);
    }
  }
]);
