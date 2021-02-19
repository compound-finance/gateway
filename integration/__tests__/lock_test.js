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

describe('lock tests', () => {
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

  test('lock asset', async () => {
    let tx = await contracts.starport.methods.lockEth().send({ value: 1e18, from: accounts[0] });
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

  test('lock eth', async () => {
    let tx = await contracts.starport.methods.lockEth().send({ value: 1e18, from: accounts[0] });
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
