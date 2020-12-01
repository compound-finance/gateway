const RLP = require('rlp');
const {ethers} = require('ethers');
const {Wallet} = require("@ethersproject/wallet");

const bi = num => {
  return BigInt(num);
};

const mantissa = num => {
  return bi(num) * bi(1e18);
}

const getHypotheticalAddr = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
}

let createAccounts = (num) => {
  return Array(num).fill(null).map(() => Wallet.createRandom());
}

// No ethereum prefix, just signs the hash of the raw digest
const sign = (msg, account) => {
  const hash = ethers.utils.keccak256(msg);
  const sk = new ethers.utils.SigningKey(account.privateKey);
  const hashArr = ethers.utils.arrayify(hash);
  const sigRaw = sk.signDigest(hashArr);
  const sig = ethers.utils.joinSignature(sigRaw);

  return {hash, sig}
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
  const authorityAccts = createAccounts(3);
  const authorityAddrs = authorityAccts.map(acct => acct.address);
  let [root, a1] = saddle.accounts;
  const testMsg = "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";


  beforeEach(async () => {
    const nonce = await web3.eth.getTransactionCount(root);
    const starportAddr = getHypotheticalAddr(accounts[0], nonce);
    const cashAddr = getHypotheticalAddr(accounts[0], nonce + 1);
    starport = await deploy('Starport', [cashAddr, authorityAddrs]);
    cash = await deploy('MockCashToken', [starportAddr, mantissa(100), a1]);

    tokenA = await deploy('FaucetToken', [mantissa(100), "tokenA", 18, "TKNA"]);
    await send(tokenA, "allocateTo", [a1, mantissa(10)]);
    await send(tokenA, "approve", [starport._address, mantissa(10)], { from: a1 });
  });

  it('should calculate quorum correctly', async () => {
    const testQuorum = async (numAuth, quorum) => expect(await call(starport, 'getQuorum',[numAuth])).numEquals(quorum);
    await testQuorum(1,1);
    await testQuorum(3,2);
    await testQuorum(5,2);
    await testQuorum(6,3);
  });

  it('should recover signer', async () => {
    const acct = authorityAccts[0];
    const {hash, sig} = sign(testMsg, acct);
    const signer = await call(starport, 'recover', [hash, sig]);
    expect(signer).toBe(acct.address);
  });

  it('should authorize message', async () => {
    const sigs = authorityAccts.map(acct => sign(testMsg, acct).sig);
    expect(await call(starport, 'isMsgAuthorized', [testMsg, authorityAddrs, sigs])).toBe(true);
  });

  it('should not authorize duplicate sigs', async () => {
    const dupAccts = Array(3).fill(authorityAccts[0]);
    const sigs = dupAccts.map(acct => sign(testMsg, acct).sig);
    await expect(call(starport, 'isMsgAuthorized', [testMsg, authorityAddrs, sigs])).rejects.toRevert('revert Duplicated sig');
  });

  it('should not authorize with too few sigs', async () => {
    const acct = authorityAccts[0];
    const sig = sign(testMsg, acct).sig;
    await expect(call(starport, 'isMsgAuthorized', [testMsg, authorityAddrs, [sig]])).rejects.toRevert('revert Below quorum threshold');
  });

  it('should not authorize with unauthorized signer', async () => {
    const badAccts = createAccounts(2);
    const sigs = badAccts.map(acct => sign(testMsg, acct).sig);
    await expect(call(starport, 'isMsgAuthorized', [testMsg, authorityAddrs, sigs])).rejects.toRevert('revert Unauthorized signer');
  });


  const ethChainType = () => {
    const byteArr = ethers.utils.toUtf8Bytes("ETH");
    const decoded = ethers.utils.defaultAbiCoder.encode(['bytes3'], [byteArr]);
    return decoded.slice(2).substring(6,0);
  }

  it('should change authorities', async ()=> {
    const chainTypeBytes = ethChainType();

    const LEN = 2;
    const newAuths = createAccounts(LEN).map(acct => acct.address);
    const paramTypes = Array(LEN).fill('address');
    const encodedAddrs = ethers.utils.defaultAbiCoder.encode(paramTypes, newAuths).slice(2);
    const notice = "0x" + chainTypeBytes + encodedAddrs;

    const sigs = authorityAccts.map(acct => sign(notice, acct).sig);

    await send(starport, 'changeAuthorities', [notice, sigs]);
    const authAfter = await call(starport, 'getAuthorities');
    expect(authAfter).toEqual(newAuths);

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
