const RLP = require('rlp');
const {ethers} = require('ethers');

const bi = num => {
  return BigInt(num);
};

const mantissa = num => {
  return bi(num) * bi(1e18);
}

const getHypotheticalAddr = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
}

const ETH_ADDRESS = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

const getLockEventTopic = () => {
  const i = new ethers.utils.Interface(["event Lock(address asset, address holder, uint amount)"]);
  return i.events.Lock.topic;
}

// TODO: test fee token
describe('Starport', () => {
  let starport;
  let cash;
  let tokenA;
  let [root, a1, a2] = saddle.accounts;

  beforeEach(async () => {
    const nonce = await web3.eth.getTransactionCount(root);
    const starportAddr = getHypotheticalAddr(accounts[0], nonce);
    const cashAddr = getHypotheticalAddr(accounts[0], nonce + 1);
    starport = await deploy('Starport', [cashAddr]);
    cash = await deploy('MockCashToken', [starportAddr, mantissa(100), a1]);

    tokenA = await deploy('FaucetToken', [mantissa(100), "tokenA", 18, "TKNA"]);
    await send(tokenA, "allocateTo", [a1, mantissa(10)]);
    await send(tokenA, "approve", [starport._address, mantissa(10)], { from: a1 });
  });

  it('should have correct references', async () => {
    expect(await call(starport, 'cash')).addrEquals(cash._address);
    expect(await call(cash, 'admin')).addrEquals(starport._address);
  });

  it('should lock asset', async () => {
    const amt = mantissa(1);
    const balBefore = bi(await call(tokenA, 'balanceOf', [a1]));
    const tx = await send(starport, 'lock', [amt, tokenA._address], { from: a1 });
    const balAfter = bi(await call(tokenA, 'balanceOf', [a1]));
    expect(balBefore - balAfter).numEquals(amt);
    expect(await call(tokenA, 'balanceOf', [starport._address])).numEquals(amt);

    expect(tx.events.Lock.returnValues).toMatchObject({
      asset: tokenA._address,
      amount: amt.toString(),
      holder: a1
      }
    );
  });

  it('should lock cash', async () => {
    const amt = mantissa(1);
    const balBefore = bi(await call(cash, 'balanceOf', [a1]));

    const tx = await send(starport, 'lock', [amt, cash._address], { from: a1 });

    const balAfter = bi(await call(cash, 'balanceOf', [a1]));
    expect(balBefore - balAfter).numEquals(amt);
    expect(await call(cash, 'balanceOf', [starport._address])).numEquals(amt);
    expect(await call(starport, 'cash')).addrEquals(cash._address);

    expect(tx.events.LockCash.returnValues).toMatchObject({
        amount: amt.toString(),
        // yieldIndex: TODO
        holder: a1
      }
    );
  });

  it('should lock ETH', async () => {
    const amt = mantissa(1);
    const bal0 = await web3.eth.getBalance(starport._address);
    expect(bal0).numEquals(0);
    const tx = await send(starport, 'lockETH', [], { from: a1, value: Number(amt) });
    const bal1 = await web3.eth.getBalance(starport._address);
    expect(bal1).numEquals(amt);

    expect(tx.events.Lock.returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        holder: a1,
        amount: amt.toString()
      }
    );
  });

  it('fallback lock ETH', async () => {
    const amt = mantissa(1);
    const bal0 = await web3.eth.getBalance(starport._address);
    expect(bal0).numEquals(0);
    const tx = await web3.eth.sendTransaction({ to: starport._address, from: a1, value: Number(amt)});
    const bal1 = await web3.eth.getBalance(starport._address);
    expect(bal1).numEquals(amt);
    expect(tx.logs[0].topics[0]).toBe(getLockEventTopic());
  });

});
