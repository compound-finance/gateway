const RLP = require('rlp');
const { ethers } = require('ethers');
const { Wallet } = require("@ethersproject/wallet");

const bi = num => {
  return BigInt(num);
};

const mantissa = num => {
  return bi(num) * bi(1e18);
}

const getHypotheticalAddr = (acct, nonce) => {
  return '0x' + web3.utils.sha3(RLP.encode([acct, nonce])).slice(12).substring(14);
}

let createAddress = () =>  Wallet.createRandom().address;

let createAccounts = (num) => {
  return Array(num).fill(null).map(() => Wallet.createRandom());
}

const newAuthNoticeBytes = num => {
  const newAuths = createAccounts(num).map(acct => acct.address);
  const paramTypes = Array(num).fill('address');
  return ethers.utils.defaultAbiCoder.encode(paramTypes, newAuths).slice(2);
}

// No ethereum prefix, just signs the hash of the raw digest
const sign = (msg, account) => {
  const hash = ethers.utils.keccak256(msg);
  const sk = new ethers.utils.SigningKey(account.privateKey);
  const hashArr = ethers.utils.arrayify(hash);
  const sigRaw = sk.signDigest(hashArr);
  const sig = ethers.utils.joinSignature(sigRaw);

  return { hash, sig }
}

//'455448'
const ethChainType = () => {
  const byteArr = ethers.utils.toUtf8Bytes("ETH");
  const decoded = ethers.utils.defaultAbiCoder.encode(['bytes3'], [byteArr]);
  return decoded.slice(2).substring(6, 0);
}

const encodeUint = (num) => ethers.utils.defaultAbiCoder.encode(['uint'], [num]).slice(2);

const encodeAddr = (addr) => ethers.utils.defaultAbiCoder.encode(['address'], [addr]).slice(2);

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

  // just the header
  // {chain type, era ID, era index, parent}
  const userMsg = "0x" + ethChainType() + encodeUint(0) + encodeUint(3) + encodeAddr(createAddress());
  const adminMsg = "0x" + ethChainType() + encodeUint(1) + encodeUint(3) + encodeAddr(createAddress());

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

  describe('Unit', () => {
    it('should have correct references', async () => {
      expect(await call(starport, 'cash')).addrEquals(cash._address);
      expect(await call(cash, 'admin')).addrEquals(starport._address);
    });

    it('should calculate quorum correctly', async () => {
      const testQuorum = async (numAuth, quorum) => expect(await call(starport, 'getQuorum', [numAuth])).numEquals(quorum);
      await testQuorum(1, 1);
      await testQuorum(3, 2);
      await testQuorum(5, 2);
      await testQuorum(6, 3);
    });

    it('should recover signer', async () => {
      const acct = authorityAccts[0];
      const { hash, sig } = sign(userMsg, acct);
      const signer = await call(starport, 'recover', [hash, sig]);
      expect(signer).toBe(acct.address);
    });

    it('should authorize message', async () => {
      const sigs = authorityAccts.map(acct => sign(userMsg, acct).sig);
      await call(starport, 'assertNoticeAuthorized', [userMsg, authorityAddrs, sigs, false]);
    });

    it('should authorize admin message', async () => {
      const sigs = authorityAccts.map(acct => sign(adminMsg, acct).sig);
      await call(starport, 'assertNoticeAuthorized', [adminMsg, authorityAddrs, sigs, true]);
    });

    it('should not authorize message without current eraID for normal notice', async () => {
      const msg = "0x" + ethChainType() + encodeUint(4) + encodeUint(1) + "1234567890";
      const sigs = authorityAccts.map(acct => sign(msg, acct).sig);
      await expect(call(starport, 'assertNoticeAuthorized', [msg, authorityAddrs, sigs, false])).rejects.toRevert('revert Notice must use current era');
    });

    it('should not authorize admin message without incremented era ID', async () => {
      const sigs = authorityAccts.map(acct => sign(userMsg, acct).sig);
      await expect(call(starport, 'assertNoticeAuthorized', [userMsg, authorityAddrs, sigs, true])).rejects.toRevert('revert Admin notice must increment era');
    });


    it('should not authorize duplicate sigs', async () => {
      const dupAccts = Array(3).fill(authorityAccts[0]);
      const sigs = dupAccts.map(acct => sign(userMsg, acct).sig);
      await expect(call(starport, 'assertNoticeAuthorized', [userMsg, authorityAddrs, sigs, false])).rejects.toRevert('revert Duplicated sig');
    });

    it('should not authorize with too few sigs', async () => {
      const acct = authorityAccts[0];
      const sig = [sign(userMsg, acct).sig];
      await expect(call(starport, 'assertNoticeAuthorized', [userMsg, authorityAddrs, sig, false])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should not authorize with unauthorized signer', async () => {
      const badAccts = createAccounts(2);
      const sigs = badAccts.map(acct => sign(userMsg, acct).sig);
      await expect(call(starport, 'assertNoticeAuthorized', [userMsg, authorityAddrs, sigs, false])).rejects.toRevert('revert Unauthorized signer');
    });
  });

  describe('Starport', () => {
    describe('changeAuth', () => {
      it('should change authorities', async () => {
        const newAuths = createAccounts(2).map(acct => acct.address);
        const paramTypes = Array(2).fill('address');
        const newAuthNoticeBytes = ethers.utils.defaultAbiCoder.encode(paramTypes, newAuths).slice(2);

        const notice = adminMsg + newAuthNoticeBytes;
        const sigs = authorityAccts.map(acct => sign(notice, acct).sig);

        expect(await call(starport, 'eraId')).numEquals(0);
        const tx = await send(starport, 'changeAuthorities', [notice, sigs]);
        expect(await call(starport, 'eraId')).numEquals(1);

        const authAfter = await call(starport, 'getAuthorities');
        expect(authAfter).toEqual(newAuths);

        const authHash = ethers.utils.keccak256(ethers.utils.solidityPack(['address[]'], [newAuths]));
        expect(tx.events.ChangeAuthorities.returnValues.authHash).toBe(authHash);

        await expect(send(starport, 'changeAuthorities', [notice, sigs])).rejects.toRevert('revert Admin notice must increment era');

      });

      it('should not change authorities with invalid signers', async () => {
        const validNotice = adminMsg + newAuthNoticeBytes(2);
        const badSigners = createAccounts(1);
        const badSigs = badSigners.map(acct => sign(validNotice, acct).sig);
        await expect(send(starport, 'changeAuthorities', [validNotice, badSigs])).rejects.toRevert('revert Unauthorized signer');

        const badNotice = "0x" + ethChainType() + newAuthNoticeBytes(0);
        const validSigs = authorityAccts.map(acct => sign(badNotice, acct).sig);
        await expect(send(starport, 'changeAuthorities', [badNotice, validSigs])).rejects.toRevert('revert New authority set can not be empty');
      });

      it('should not change authorities with wrong chain type', async () => {
        const notice = "0x" + "123456" + encodeUint(1) + encodeUint(4) + newAuthNoticeBytes(2);
        const sigs = authorityAccts.map(acct => sign(notice, acct).sig);

        await expect(send(starport, 'changeAuthorities', [notice, sigs])).rejects.toRevert('revert Invalid Chain Type');
      });

      it('should revert if notice has wrong number of bytes', async () => {
        const notice = adminMsg + newAuthNoticeBytes(2) + "abcd"
        const sigs = authorityAccts.map(acct => sign(notice, acct).sig);
        await expect(send(starport, 'changeAuthorities', [notice, sigs])).rejects.toRevert('revert Excess bytes');
      });
    });

    describe('lock', () => {
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
        });
      });

      it('fallback lock ETH', async () => {
        const amt = mantissa(1);
        const bal0 = await web3.eth.getBalance(starport._address);
        expect(bal0).numEquals(0);
        const tx = await web3.eth.sendTransaction({ to: starport._address, from: a1, value: Number(amt) });
        const bal1 = await web3.eth.getBalance(starport._address);
        expect(bal1).numEquals(amt);
        expect(tx.logs[0].topics[0]).toBe(getLockEventTopic());
      });
    });

    describe.only('unlock', () => {
      it('should unlock asset', async () => {
        const [asset, account] = createAccounts(2).map(a => a.address);
        const amount = '1000';
        const notice = userMsg + encodeAddr(account) + encodeUint(amount) + encodeAddr(asset);

        const sigs = authorityAccts.map(acct => sign(notice, acct).sig);
        const tx = await send(starport, 'unlock', [notice, sigs], { from: a1 });
        expect(tx.events.Unlock.returnValues).toMatchObject({asset, account, amount});
      });

      // unsigned notice from compound chain
      // placeholder for an actual end to end test. 
      it.skip('should do end to end test', async() => {
        const e2eMsg = "0x455448" + //chainid
        "0000000000000000000000000000000000000000000000000000000000000000" +  //era id
        "0000000000000000000000000000000000000000000000000000000000000000" + // eraIdx
        "3030303030303030303030303030303030303030303030303030303030303030" + //parent
        "0000000000000000000000000101010101010101010101010101010101010101" + //acct
        "0000000000000000000000000000000000000000000000000000000000000032" + // amt
        "0000000000000000000000002020202020202020202020202020202020202020" // asset


        const sigs = authorityAccts.map(acct => sign(e2eMsg, acct).sig);
        const tx = await send(starport, 'unlock', [e2eMsg, sigs], { from: a1 });
        console.log(tx.events.Unlock.returnValues);
        // expect(tx.events.Unlock.returnValues).toMatchObject({asset, account, amount});
      })

    });
  });
});
