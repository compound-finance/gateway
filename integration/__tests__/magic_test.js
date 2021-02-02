const {
  initialize,
  teardown
} = require('../util/test');
const {
  getEventData,
  findEvent,
  sendAndWaitForEvents,
  waitForEvent
} = require('../util/substrate');
const { log, error } = require('../util/log');
const { getEventValues } = require('../util/ethereum');

/* This is probably going to be permanently skipped.
   We might want to keep it since it shows the non-scenario
   way to build an integration test.
*/
describe('magic extract and goldie unlocks', () => {
  let accounts,
    ashley,
    api,
    bert,
    contracts,
    ctx,
    keyring,
    provider,
    web3;

  beforeEach(async () => {
    ({
      accounts,
      ashley,
      api,
      bert,
      contracts,
      ctx,
      keyring,
      provider,
      web3,
    } = await initialize());
  }, 600000 /* 10m */);
  afterEach(() => teardown(ctx));

  // There's now a lot wrong with this test (e.g. the Starport doesn't even
  // have a cash token balance to extract out). It should be removed with magic
  // extract in general.
  test.skip('magic extraction', async () => {
    let trxReq = "(magic-extract 1000 eth:0xc00e94cb662c3520282e6f5717214004a7f26888)";
    let sig = { Eth: await web3.eth.sign(trxReq, accounts[0]) };
    let cashBalancePrior = await api.query.cash.cashBalance({ Eth: "0xc00e94cb662c3520282e6f5717214004a7f26888" });
    let call = api.tx.cash.execTrxRequest(trxReq, sig);

    let events = await sendAndWaitForEvents(call, api, false);

    let magicExtractEvent = findEvent(events, 'cash', 'MagicExtract');

    expect(magicExtractEvent).toBeDefined();
    expect(getEventData(magicExtractEvent)).toEqual({
      AssetAmount: 1000,
      ChainAccount: {
        Eth: "0xc00e94cb662c3520282e6f5717214004a7f26888",
      },
      Notice: {
        CashExtractionNotice: {
          Eth: {
            id: [expect.any(Number), 0],
            parent: "0x0000000000000000000000000000000000000000000000000000000000000000",
            account: "0xc00e94cb662c3520282e6f5717214004a7f26888",
            amount: 1000,
            cash_index: 1000
          }
        }
      }
    });

    let cashBalancePost = await api.query.cash.cashBalance({ Eth: "0xc00e94cb662c3520282e6f5717214004a7f26888" });

    expect(cashBalancePost.unwrap() - cashBalancePrior.unwrapOr(0)).toEqual(1000);

    let signedNotice = findEvent(events, 'cash', 'SignedNotice');

    expect(signedNotice).toBeDefined();
    let eventData = getEventData(signedNotice);
    expect(eventData).toHaveProperty('ChainId', "Eth");
    expect(eventData).toHaveProperty('NoticeId', [0, 0]);
    expect(eventData).toHaveProperty('EncodedNotice');
    expect(eventData).toHaveProperty('ChainSignatureList');
    let notice = eventData['EncodedNotice'];
    let noticeSigs = eventData['ChainSignatureList'];
    expect(noticeSigs).toHaveProperty('Eth');

    // TODO: This should probably be "unlockCash"
    let tx = await contracts.starport.methods.unlock(notice, noticeSigs['Eth'].map((x) => x[1])).send({ from: accounts[0] });

    let unlockEvent = tx.events['Unlock'];
    expect(notice).toBeDefined();

    expect(getEventValues(unlockEvent)).toEqual({
      account: "0xc00e94Cb662C3520282E6f5717214004A7f26888",
      amount: "1000",
      asset: "0x00000000000000000000000000000000000003e8", // uhhh?
    });

    // TODO: Update once Starport actually unlocks tokens
    expect(await contracts.cashToken.methods.balanceOf("0xc00e94Cb662C3520282E6f5717214004A7f26888").call()).toEqual("0");

    // Everything's good.
  }, 600000 /* 10m */);

  test('lock asset', async () => {
    let tx = await contracts.starport.methods.lockETH().send({ value: 1e18, from: accounts[0] });
    let goldieLocksEvent = await waitForEvent(api, 'cash', 'GoldieLocks', false, ['cash', 'FailedProcessingEthEvent']);

    expect(getEventData(goldieLocksEvent)).toEqual({
      ChainAccount: {
        Eth: accounts[0].toLowerCase(),
      },
      ChainAsset: {
        Eth: "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      },
      AssetAmount: "0x00000000000000000de0b6b3a7640000",
    });

    // Everything's good.
  }, 600000 /* 10m */);

  test('asset', async () => {
    let tx = await contracts.starport.methods.lockETH().send({ value: 1e18, from: accounts[0] });
    let goldieLocksEvent = await waitForEvent(api, 'cash', 'GoldieLocks', false, ['cash', 'FailedProcessingEthEvent']);

    expect(getEventData(goldieLocksEvent)).toEqual({
      ChainAccount: {
        Eth: accounts[0].toLowerCase(),
      },
      ChainAsset: {
        Eth: "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
      },
      AssetAmount: "0x00000000000000000de0b6b3a7640000",
    });

    // Everything's good.
  }, 600000 /* 10m */);

  // TODO: Submit trx to Starport and check event logs

  // TODO: Submit extrinsic to Compound Chain and collect notices

  // TODO: Submit notices to Starport
});
