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
  let alice_init_eth_key = "0x6a72a2f14577d9cd0167801efdd54a07b40d2b61";
  
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
    const [[val_id_0, chainKeys_0]] = await api.query.cash.validators.entries();
    
    const val_0 = toSS58(val_id_0.args[0]);
    expect(val_0).toEqual(alice_init_id);
    
    const eth_address = u8aToHex(chainKeys_0.unwrap().eth_address);
    expect(eth_address).toEqual(alice_init_eth_key);

    const [auths] = await api.query.babe.authorities();
    const babe_id_0 = toSS58(auths[0]);
    expect(babe_id_0).toEqual(alice_init_id);

    // todo: query and assert grandpa state via the GRANDPA_AUTHORITIES_KEY https://www.shawntabrizi.com/substrate/querying-substrate-storage-via-rpc/

  }, 600000 /* 10m */);
});
