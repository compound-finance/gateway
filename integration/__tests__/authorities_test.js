const {
  initialize,
  teardown
} = require('../util/test');
const { log, error } = require('../util/log');
const { u8aToHex } = require('@polkadot/util');
const { signAndSend } = require('../util/substrate');

// relies upon short session period runtime/src for debug mode 
async function waitUntilSession(num, api) {
  const timer = ms => new Promise(res => setTimeout(res, ms));
  const checkIdx = async () => {
    const idx = (await api.query.session.currentIndex()).toNumber();
    if (idx <= num) {
      await timer(2000);
      await checkIdx();
    }
  };
  await checkIdx();
}

/* This is probably going to be permanently skipped.
   We might want to keep it since it shows the non-scenario
   way to build an integration test.
*/
describe('authorities tests', () => {
  let accounts,
    ashley,
    api,
    bert,
    contracts,
    ctx,
    keyring,
    provider,
    web3;

  let aliceInitId = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
  let aliceInitEthKey = "0x6a72a2f14577d9cd0167801efdd54a07b40d2b61";
  let bobInitId = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty";
  let bobInitEthKey = "0x8ad1b2918c34ee5d3e881a57c68574ea9dbecb81";

  // https://github.com/paritytech/substrate/blob/caff191bc1bfa48688037c6024ee3a2a1cbeb084/primitives/finality-grandpa/src/lib.rs#L62
  let grandpaStorageKey = [58, 103, 114, 97, 110, 100, 112, 97, 95, 97, 117, 116, 104, 111, 114, 105, 116, 105, 101, 115];


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

  const toSS58 = (arr) => keyring.encodeAddress(new Uint8Array(arr.buffer));

  // change from alice and bob to just alice
  test('authorities', async () => {
    const auths = await api.query.cash.validators.entries();

    // array of [valIdss58, ethAddress]
    const authData = auths.map(([valIdRaw, chainKeys]) =>
      [
        toSS58(valIdRaw.args[0]),
        u8aToHex(chainKeys.unwrap().eth_address)
      ]
    );
    expect(authData).toEqual(
      [[bobInitId, bobInitEthKey], [aliceInitId, aliceInitEthKey]]
    );
    const newAuths = [[keyring.decodeAddress(aliceInitId), { eth_address: aliceInitEthKey }]];
    const sudoPair = keyring.addFromUri('//Alice');
    expect(sudoPair.address).toEqual(toSS58(await api.query.sudo.key()));

    const unsub = await api.tx.sudo.sudo(api.tx.cash.changeAuthorities(newAuths)).signAndSend(sudoPair, ({ events = [], status }) => {
      console.log('Transaction status:', status.type);
      if (status.isInBlock) {
        console.log('Completed at block hash', status.asInBlock.toHex());
        console.log('Events:');
        events.forEach(({ phase, event: { data, method, section } }) => {
          console.log('HERE:', phase.toString(), `${section} ${method} `, data.toString());
        });
      } else if (status.isFinalized) {
        console.log("FINALIZED:", events);// never hit this
        unsub();
      }
    });
    // await signAndSend(tx, sudoPair, api);

    console.log("next sess idx", (await api.query.cash.nextSessionIndex()).toNumber());
    const queued = await api.query.cash.nextValidators.entries();
    expect(queued.length).toEqual(1);
    // const [pendingAuth0, pendingChainkeys0] = queued[0];
    // expect(toSS58(pendingAuth0.args[0])).toEqual(aliceInitId);
    // expect(u8aToHex(pendingChainkeys0.unwrap().eth_address)).toEqual(aliceInitEthKey);

    await waitUntilSession(4, api);
    const nextAuths = await api.query.cash.nextValidators.entries();
    expect(nextAuths).toEqual([]);

    const afterAuthRaw = await api.query.cash.validators.entries();
    const afterAuths = afterAuthRaw.map(([valIdRaw, chainKeys]) =>
      [
        toSS58(valIdRaw.args[0]),
        u8aToHex(chainKeys.unwrap().eth_address)
      ]
    );

    console.log("next sess idx", (await api.query.cash.nextSessionIndex()).toNumber());

    expect(afterAuths).toEqual(
      [[aliceInitId, aliceInitEthKey]]
    );

    // expect([]).toEqual(await api.query.cash.nextValidators.entries());

    // const newValidators = await api.query.session.validators().map(e => toSS58(e));
    // console.log(newValidators);

    // todo: query and assert grandpa/babe state via the GRANDPA_AUTHORITIES_KEY https://www.shawntabrizi.com/substrate/querying-substrate-storage-via-rpc/
    // const gpas = await api.rpc.state.getStorage(u8aToHex(grandpaStorageKey));
    // console.log("Grandpa key", gpas.unwrap());

  }, 600000 /* 10m */);
});
