const {
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let extract_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } },
    { token: "bat", balances: { bert: 25000 } },
    { token: "usdc", balances: { ashley: 100_000 } },
    { token: "comp", balances: { bert: 200 } },
  ]
};

async function lockUSDC({ ashley, zrx }) {
  await ashley.lock(100, zrx);
  expect(await ashley.tokenBalance(zrx)).toEqual(900);
  expect(await ashley.chainBalance(zrx)).toEqual(100);
}

buildScenarios('Extract Scenarios', extract_scen_info, { beforeEach: lockUSDC }, [
  {
    name: "Extract Collateral",
    scenario: async ({ ashley, zrx, chain, starport, log }) => {
      let notice = getNotice(await ashley.extract(50, zrx));
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await ashley.tokenBalance(zrx)).toEqual(900);
      await starport.invoke(notice, signatures);
      expect(await ashley.tokenBalance(zrx)).toEqual(950);
      expect(await ashley.chainBalance(zrx)).toEqual(50);
    }
  },
  {
    name: "Extract via Starport Action",
    scenario: async ({ ashley, zrx, chain, starport, log }) => {
      await ashley.execTrxRequest(ashley.extractTrxReq(50, zrx));
      let notice = await chain.waitForNotice();
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await ashley.tokenBalance(zrx)).toEqual(900);
      await starport.invoke(notice, signatures);
      expect(await ashley.tokenBalance(zrx)).toEqual(950);
      expect(await ashley.chainBalance(zrx)).toEqual(50);
    }
  },
  {
    name: "Extract Cash",
    scenario: async ({ ashley, zrx, chain, starport, cash, log }) => {
      let notice = getNotice(await ashley.extract(20, cash));
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await cash.getCashPrincipal(ashley)).toEqual(0);
      expect(await ashley.tokenBalance(cash)).toEqual(0);
      let tx = await starport.invoke(notice, signatures);
      expect(await ashley.tokenBalance(cash)).toBeCloseTo(20, 4);
      expect(await ashley.cash()).toBeCloseTo(-20, 4);
    }
  },
  {
    name: "Extract Cash Torrey",
    beforeEach: null,
    scenario: async ({ bert, bat, chain, starport, cash, log }) => {
      await bert.lock(25000, bat);
      let notice = getNotice(await bert.extract(5, cash));
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await cash.getCashPrincipal(bert)).toEqual(0);
      expect(await bert.tokenBalance(cash)).toEqual(0);
      let tx = await starport.invoke(notice, signatures);
      expect(await bert.tokenBalance(cash)).toBeCloseTo(5, 4);
      expect(await bert.cash()).toBeCloseTo(-5, 4);
    }
  },
  {
    name: "Extract Comp Torrey",
    beforeEach: null,
    scenario: async ({ ashley, bert, usdc, comp, chain, starport, cash, log }) => {
      // User A Supplied 100k USDC
      await ashley.lock(100_000, usdc);
      // User B Supplied 200 COMP
      await bert.lock(200, comp);
      // User A downloaded 50 COMP
      let notice = getNotice(await ashley.extract(50, comp));
      let signatures = await chain.getNoticeSignatures(notice);

      expect(await ashley.tokenBalance(comp)).toEqual(0);
      let tx = await starport.invoke(notice, signatures);
      let ashleyComp = await ashley.tokenBalance(comp);
      let ashleyCash = await ashley.cash();
      let ashleyLiquidity = await ashley.liquidity();
      expect(ashleyComp).toEqual(50);
      expect(ashleyCash).toBeCloseTo(-0.0003809713723277, 4);
      expect(ashleyLiquidity).toBeCloseTo(64700, -3);
    }
  },
  {
    name: "Extract Cash Max",
    scenario: async ({ ashley, bert, zrx, chain, starport, cash }) => {
      await ashley.transfer(10, cash, bert);
      expect(await bert.cash()).toBeCloseTo(10, 4);
      let notice = getNotice(await bert.extract('Max', cash));
      let signatures = await chain.getNoticeSignatures(notice);

      expect(await ashley.cash()).toBeCloseTo(-10.01, 4);
      expect(await bert.cash()).toEqual(0, 4);
      expect(await bert.tokenBalance(cash)).toEqual(0);
      await starport.invoke(notice, signatures);
      expect(await bert.tokenBalance(cash)).toBeCloseTo(10, 4);
    }
  }
]);
