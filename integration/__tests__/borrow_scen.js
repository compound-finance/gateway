const {
  buildScenarios,
  sleep,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let borrow_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 10_000_000 }, supply_cap: 10_000_000 },
    { token: "bat", balances: { bert: 2_000_000 }, supply_cap: 2_000_000 }
  ],
};

async function lockZRX({ ashley, bert, bat, zrx }) {
  await ashley.lock(10_000_000, zrx);
  await bert.lock(2_000_000, bat);
  expect(await ashley.tokenBalance(zrx)).toEqual(0);
  expect(await ashley.chainBalance(zrx)).toEqual(10_000_000);
  expect(await bert.chainBalance(bat)).toEqual(2_000_000);
}

buildScenarios('Borrow Scenarios', borrow_scen_info, { beforeEach: lockZRX }, [
  {
    name: "Borrow BAT and Garner Interest",
    notes: "This test allows arbitrary time passage, and thus has some estimation",
    scenario: async ({ ashley, bert, bat, zrx, chain, starport, log, cash }) => {
      let cashBalance0 = await ashley.chainBalance(cash);
      let cashIndex0 = await chain.cashIndex();
      let cash0 = await ashley.cash();

      expect(cash0).toEqual(0);

      // Ashley Borrows BAT
      let notice = getNotice(await ashley.extract(1_000_000, bat));

      // Check totals
      expect(await zrx.totalChainSupply()).toEqual(10_000_000);
      expect(await zrx.totalChainBorrows()).toEqual(0);
      expect(await bat.totalChainSupply()).toEqual(2_000_000);
      expect(await bat.totalChainBorrows()).toEqual(1_000_000);

      // Pull BAT from the Starport
      let signatures = await chain.getNoticeSignatures(notice);
      await starport.invoke(notice, signatures);

      // Check totals
      expect(await ashley.tokenBalance(bat)).toEqual(1_000_000);
      expect(await ashley.chainBalance(zrx)).toEqual(10_000_000);
      expect(await ashley.chainBalance(bat)).toEqual(-1_000_000);

      // See that we've had _any_ cash interest accrued (assume it's less than a dime)
      let cashBalance1 = await ashley.chainBalance(cash);
      let cashIndex1 = await chain.cashIndex();
      let cash1 = await ashley.cash();
      expect(cash1).toBeLessThan(0);
      expect(cash1).toBeGreaterThan(-0.10);

      await sleep(20000);

      // See that we've had _more_ cash interest accrued (assume it's less than a quarter)
      let cashBalance2 = await ashley.chainBalance(cash);
      let cashIndex2 = await chain.cashIndex();
      let cash2 = await ashley.cash();

      // Nothing is exact here.
      expect(cash2).toBeLessThan(cash1);
      expect(cash2).toBeGreaterThan(-0.25);
    }
  }
]);
