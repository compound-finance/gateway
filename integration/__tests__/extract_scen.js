const {
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let extract_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
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
    only: true,
    name: "Extract Cash",
    scenario: async ({ ashley, zrx, chain, starport, cash, log }) => {
      let notice = getNotice(await ashley.extract(20, cash));
      console.log({notice});
      console.log(notice.Notice);
      console.log(notice.Notice.CashExtractionNotice);
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await cash.getCashPrincipal(ashley)).toEqual(0);
      expect(await ashley.tokenBalance(cash)).toEqual(0);
      let tx = await starport.invoke(notice, signatures);
      log({tx});
      log(tx.events.UnlockCash);
      // expect(await cash.getCashPrincipal(ashley)).toEqual(5000);
      expect(await ashley.tokenBalance(cash)).toEqual(20);
      expect(await ashley.chainBalance(cash)).toEqual(-20);
    }
  },
  {
    skip: true,
    name: "Extract Cash Max",
    scenario: async ({ ashley, zrx, chain, starport, cash }) => {
      // TODO: Make sure user has Cash to begin scenario
      let notice = getNotice(await ashley.extract('Max', cash));
      let signatures = await chain.getNoticeSignatures(notice);
      expect(await cash.getCashPrincipal(ashley)).toEqual(0);
      expect(await ashley.tokenBalance(cash)).toEqual(0);
      await starport.invoke(notice, signatures);
      expect(await cash.getCashPrincipal(ashley)).toEqual(5000);
      expect(await ashley.tokenBalance(cash)).toEqual(50);
      expect(await ashley.chainBalance(cash)).toEqual(-50);
    }
  }
]);
