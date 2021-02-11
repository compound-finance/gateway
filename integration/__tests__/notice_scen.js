const {
  years,
  buildScenarios,
} = require('../util/scenario');
const { getNotice } = require('../util/substrate');

let notice_scen_info = {
  tokens: [
    { token: "zrx", balances: { ashley: 1000 } }
  ],
};

async function lockUSDC({ ashley, zrx }) {
  await ashley.lock(100, zrx);
  expect(await ashley.tokenBalance(zrx)).toEqual(900);
  expect(await ashley.chainBalance(zrx)).toEqual(100);
}

buildScenarios('Notice Scenarios', notice_scen_info, { beforeEach: lockUSDC }, [
  {
    name: "Extract by Notice Chain",
    scenario: async ({ ashley, zrx, chain, starport }) => {
      let notice0 = getNotice(await ashley.extract(50, zrx));
      let notice1 = getNotice(await ashley.extract(25, zrx));
      let signatures1 = await chain.getNoticeSignatures(notice1);

      expect(await ashley.tokenBalance(zrx)).toEqual(900);

      await starport.invoke(notice1, signatures1);
      expect(await ashley.tokenBalance(zrx)).toEqual(925);
      expect(await ashley.chainBalance(zrx)).toEqual(25);

      await starport.invokeChain(notice0, [notice1]);
      expect(await ashley.tokenBalance(zrx)).toEqual(975);
      expect(await ashley.chainBalance(zrx)).toEqual(25);
    }
  },
  {
    name: "Extract by Pulled Notice Chain",
    scenario: async ({ ashley, zrx, chain, starport }) => {
      let notice0 = getNotice(await ashley.extract(50, zrx));
      let notice1 = getNotice(await ashley.extract(25, zrx));
      let notice2 = getNotice(await ashley.extract(10, zrx));
      let signatures2 = await chain.getNoticeSignatures(notice2);

      expect(await ashley.tokenBalance(zrx)).toEqual(900);

      await starport.invoke(notice2, signatures2);
      expect(await ashley.tokenBalance(zrx)).toEqual(910);
      expect(await ashley.chainBalance(zrx)).toEqual(15);

      let notices = await chain.getNoticeChain(notice0);

      await starport.invokeChain(notice0, notices);
      expect(await ashley.tokenBalance(zrx)).toEqual(960);
      expect(await ashley.chainBalance(zrx)).toEqual(15);
    }
  }
]);
