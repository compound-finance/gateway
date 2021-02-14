const {
  buildScenarios,
  sleep,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let borrow_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } },
    { token: "bat", balances: { bert: 1000 } }
  ],
};

async function lockZRX({ ashley, bert, bat, zrx }) {
  await ashley.lock(100, zrx);
  await bert.lock(100, bat);
  expect(await ashley.tokenBalance(zrx)).toEqual(900);
  expect(await ashley.chainBalance(zrx)).toEqual(100);
  expect(await bert.chainBalance(bat)).toEqual(100);
}

buildScenarios('Borrow Scenarios', borrow_scen_info, { beforeEach: lockZRX }, [
  {
    name: "Borrow BAT",
    scenario: async ({ ashley, bert, bat, zrx, chain, starport, log, cash }) => {
      console.log("ZRX: " + zrx.ethAddress());
      console.log("BAT: " + bat.ethAddress());
      let cashBalance0 = await ashley.chainBalance(cash);
      let cashIndex0 = await chain.cashIndex();
      let cash0 = await ashley.cash();

      let notice = getNotice(await ashley.extract(50, bat));

      // Check totals
      expect(await zrx.totalChainSupply()).toEqual(100);
      expect(await zrx.totalChainBorrows()).toEqual(0);
      expect(await bat.totalChainSupply()).toEqual(100);
      expect(await bat.totalChainBorrows()).toEqual(50);

      // TODO: Extract from Starport
      // let signatures = await chain.getNoticeSignatures(notice);
      // expect(await ashley.tokenBalance(bat)).toEqual(900);
      // await starport.invoke(notice, signatures);
      // expect(await ashley.tokenBalance(bat)).toEqual(950);
      expect(await ashley.chainBalance(zrx)).toEqual(100);
      expect(await ashley.chainBalance(bat)).toEqual(-50);
      let cashBalance1 = await ashley.chainBalance(cash);
      let cashIndex1 = await chain.cashIndex();
      let cash1 = await ashley.cash();
      await sleep(20000);
      let cashBalance2 = await ashley.chainBalance(cash);
      let cashIndex2 = await chain.cashIndex();
      let cash2 = await ashley.cash();
      log({cashBalance0, cashBalance1, cashBalance2});
      log({cashIndex0, cashIndex1, cashIndex2});
      log({cash0, cash1, cash2});
      log([cashIndex0.toString(), cashIndex1.toString(), cashIndex2.toString()]);
    }
  }
]);
