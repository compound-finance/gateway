const {
  initialize,
  teardown
} = require('../util/test');
const { log, error } = require('../util/log');

const { u8aToHex } = require('@polkadot/util');

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
    
  let alice_init_id = "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY";
  let bob_init_id = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty";
  let alice_init_eth_key = "0x6a72a2f14577d9cd0167801efdd54a07b40d2b61";
  let bob_init_eth_key = "0x8ad1b2918c34ee5d3e881a57c68574ea9dbecb81";
  
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

  test('authorities', async () => {
    const auths = await api.query.cash.validators.entries();
    
    // array of [valIdss58, ethAddress]
    const auth_data = auths.map(([valIdRaw, chainKeys]) => 
      [
        toSS58(valIdRaw.args[0]),
        u8aToHex(chainKeys.unwrap().eth_address)
      ]
    );
    expect(auth_data).toEqual(
      [[bob_init_id, bob_init_eth_key], [alice_init_id, alice_init_eth_key]]
    );

    // todo: query and assert grandpa state via the GRANDPA_AUTHORITIES_KEY https://www.shawntabrizi.com/substrate/querying-substrate-storage-via-rpc/

  }, 600000 /* 10m */);
});
