const {
  buildScenarios
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let set_future_yield_scen_info = {
  validators: ['alice', 'bob']
};

buildScenarios('Set Future Yield Scenarios', set_future_yield_scen_info, [
  {
    name: 'Set a future yield',
    scenario: async ({ ashley, zrx, starport, cash, chain, ctx }) => {
      expect(await cash.nextCashYieldStart(zrx)).toEqual('0');
      expect(await cash.getNextCashYieldAndIndex(zrx)).toEqual({
        yield: '0',
        index: '0'
      });
      let futureDate = Date.now() + (2 * 24 * 60 * 60 * 1000);
      let extrinsic = ctx.api().tx.cash.setYieldNext(1000, futureDate);
      let { notice } = await starport.executeProposal("Set Future Yield", [extrinsic], { awaitNotice: true, awaitEvent: false });
      let signatures = await chain.getNoticeSignatures(notice, { signatures: 2 });
      await starport.invoke(notice, signatures);
      expect(await cash.nextCashYieldStart(zrx)).toEqual(futureDate.toString());
      expect((await cash.getNextCashYieldAndIndex(zrx)).yield).toEqual('1000');
      expect(Number((await cash.getNextCashYieldAndIndex(zrx)).index)).toBeGreaterThan(0);
      // TODO: Advance chain time and check yield and index afterwards
    }
  }
]);
