const ABICoder = require("web3-eth-abi");
const {
  bigInt,
  mantissa,
  getNextContractAddress,
  nRandomWallets,
  nRandomAuthorities,
  replaceByte,
  sign,
  signAll,
  ETH_HEADER,
  ETH_ADDRESS
} = require('./utils');

// TODO: test fee token
describe('Starport', () => {
  let starport;
  let cash;
  let tokenA;
  let [root, account1, account2] = saddle.accounts;

  const authorityWallets = nRandomWallets(3);
  const authorityAddresses = authorityWallets.map(acct => acct.address);

  let eraId;
  let eraIndex;
  let parentHash;

  function buildNotice(call, opts = {}) {
    // Set new era
    eraIndex += 1;

    const eraHeader = ABICoder.encodeParameters(['uint256', 'uint256', 'bytes32'], [opts.eraId || eraId, opts.eraIndex || eraIndex, opts.parentHash || parentHash]);
    const encodedCall = call.encodeABI();;

    let encoded = `${ETH_HEADER}${eraHeader.slice(2)}${encodedCall.slice(2)}`;

    // Set new parent hash
    parentHash = web3.utils.keccak256(encoded);

    return encoded;
  }

  let testUnlockNotice;
  let testChangeAuthoritiesNotice;

  beforeEach(async () => {
    const rootNonce = await web3.eth.getTransactionCount(root);
    const cashAddress = getNextContractAddress(root, rootNonce + 1);

    starport = await deploy('StarportHarness', [cashAddress, authorityAddresses]);
    cash = await deploy('MockCashToken', [starport._address, mantissa(100), account1]);

    tokenA = await deploy('FaucetToken', [mantissa(100), "tokenA", 18, "TKNA"]);
    await send(tokenA, "allocateTo", [account1, mantissa(10)]);
    await send(tokenA, "approve", [starport._address, mantissa(10)], { from: account1 });

    eraId = 0;
    eraIndex = 0;
    parentHash = "0x0000000000000000000000000000000000000000000000000000000000000000";

    testUnlockNotice = buildNotice(starport.methods.unlock(tokenA._address, 1000, accounts[2]));
    testChangeAuthoritiesNotice = buildNotice(starport.methods.changeAuthorities([accounts[2], accounts[3]]));
  });

  describe('Unit Tests', () => {
    it('should have correct references', async () => {
      expect(await call(starport, 'cash')).toMatchAddress(cash._address);
      expect(await call(cash, 'admin')).toMatchAddress(starport._address);
    });
  });

  describe('#getQuorum_', () => {
    it('should calculate quorum correctly', async () => {
      const testQuorum = async (authCount, quorum) =>
        expect(await call(starport, 'getQuorum_', [authCount])).toEqualNumber(quorum);

      await testQuorum(1, 1);
      await testQuorum(3, 2);
      await testQuorum(5, 2);
      await testQuorum(6, 3);
    });
  });

  describe('#recover', () => {
    it('should recover signer', async () => {
      const authority0 = authorityWallets[0];
      const { hash, signature } = sign(testUnlockNotice, authority0);
      const signer = await call(starport, 'recover_', [hash, signature]);
      expect(signer).toMatchAddress(authority0.address);
    });

    // Should we handle this case?
    it.todo('should recover EIP-155 signature');
  });

  describe('#lock', () => {
    it('should lock an asset', async () => {
      const lockAmount = mantissa(1);
      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, tokenA._address], { from: account1 });
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenA, 'balanceOf', [starport._address])).toEqualNumber(lockAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenA._address,
        amount: lockAmount.toString(),
        holder: account1
      });
    });

    it('should lock Cash', async () => {
      const lockAmount = mantissa(1);
      const balancePre = bigInt(await call(cash, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, cash._address], { from: account1 });
      const balancePost = bigInt(await call(cash, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(cash, 'balanceOf', [starport._address])).toEqualNumber(lockAmount);
      expect(await call(starport, 'cash')).toMatchAddress(cash._address);

      expect(tx.events.LockCash.returnValues).toMatchObject({
        amount: lockAmount.toString(),
        // yieldIndex: TODO
        holder: account1
      });
    });

    it('should lock Eth', async () => {
      const lockAmount = mantissa(1);
      const starportEthPre = await web3.eth.getBalance(starport._address);

      expect(starportEthPre).toEqualNumber(0);

      const tx = await send(starport, 'lockEth', [], { from: account1, value: Number(lockAmount) });
      const starportEthPost = await web3.eth.getBalance(starport._address);

      expect(starportEthPost).toEqualNumber(lockAmount);
      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        holder: account1,
        amount: lockAmount.toString()
      });
    });

    it('fallback lock Eth', async () => {
      const lockAmount = mantissa(1);
      const starportEthPre = await web3.eth.getBalance(starport._address);

      expect(starportEthPre).toEqualNumber(0);

      const tx = await web3.eth.sendTransaction({ to: starport._address, from: account1, value: Number(lockAmount) });
      const starportEthPost = await web3.eth.getBalance(starport._address);

      expect(starportEthPost).toEqualNumber(lockAmount);

      let events = await starport.getPastEvents("allEvents", { fromBlock: tx.blockNumber, toBlock: tx.blockNumber })
      expect(events[0].event).toEqual('Lock');
      expect(events[0].returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        holder: account1,
        amount: lockAmount.toString()
      });
    });
  });

  describe('#checkNoticeAuthorized', () => {
    it('should authorize message', async () => {
      const signatures = signAll(testUnlockNotice, authorityWallets);
      await call(starport, 'checkNoticeAuthorized_', [testUnlockNotice, authorityAddresses, signatures]);
    });

    it('should not authorize duplicate sigs', async () => {
      const duplicateAccounts = Array(3).fill(authorityWallets[0]);
      const signatures = signAll(testUnlockNotice, duplicateAccounts);
      await expect(call(starport, 'checkNoticeAuthorized_', [testUnlockNotice, authorityAddresses, signatures])).rejects.toRevert('revert Duplicated authority signer');
    });

    it('should not authorize with too few signatures', async () => {
      const signatures = [sign(testUnlockNotice, authorityWallets[0]).signature];
      await expect(call(starport, 'checkNoticeAuthorized_', [testUnlockNotice, authorityAddresses, signatures])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should not authorize with an unauthorized signer', async () => {
      const badAccounts = nRandomWallets(2);
      const signatures = signAll(testUnlockNotice, badAccounts);
      await expect(call(starport, 'checkNoticeAuthorized_', [testUnlockNotice, authorityAddresses, signatures])).rejects.toRevert('revert Unauthorized authority signer');
    });
  });

  describe('#invoke', () => {
    it('should invoke simple signed message', async () => {
      let notice = buildNotice(starport.methods.count_());
      let signatures = signAll(notice, authorityWallets);

      expect(await call(starport, 'invoke', [notice, signatures])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
    });

    it('should invoke multiple notices', async () => {
      let notice0 = buildNotice(starport.methods.count_());
      let signatures0 = signAll(notice0, authorityWallets);

      expect(await call(starport, 'invoke', [notice0, signatures0])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice0, signatures0]);

      let notice1 = buildNotice(starport.methods.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
    });

    it('should not authorize message without current eraId', async () => {
      let notice = buildNotice(starport.methods.count_(), { eraId: 1 });
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Notice must use current era');
    });

    it('should not authorize message without missing signatures', async () => {
      let notice = buildNotice(starport.methods.count_());

      await expect(call(starport, 'invoke', [notice, []])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should fail with invalid magic header [0]', async () => {
      let notice = replaceByte(buildNotice(starport.methods.count_()), 0, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[0]');
    });

    it('should fail with invalid magic header [1]', async () => {
      let notice = replaceByte(buildNotice(starport.methods.count_()), 1, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[1]');
    });

    it('should fail with invalid magic header [2]', async () => {
      let notice = replaceByte(buildNotice(starport.methods.count_()), 2, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[2]');
    });

    it('should fail with invalid magic header [3]', async () => {
      let notice = replaceByte(buildNotice(starport.methods.count_()), 3, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[3]');
    });

    it('fail when passed twice', async () => {
      let notice = buildNotice(starport.methods.count_());
      let signatures = signAll(notice, authorityWallets);

      await send(starport, 'invoke', [notice, signatures]);
      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Notice can not be reused');
    });

    it('fail when passed twice', async () => {
      let notice = buildNotice(starport.methods.count_()).slice(0, 99);
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Must have full header');
    });

    it.todo('invalid call');
    it.todo('call reverts');
  });

  describe('#unlock', () => {
    // TODO: Check external caller
    // TODO: Check insufficient token balance, etc

    it('should unlock asset', async () => {
      await tokenA.methods.transfer(starport._address, 1500).send({ from: root });

      expect(Number(await tokenA.methods.balanceOf(starport._address).call())).toEqual(1500);
      expect(Number(await tokenA.methods.balanceOf(account2).call())).toEqual(0);

      const tx = await send(starport, 'unlock_', [tokenA._address, 1000, account2]);

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenA._address,
        account: account2,
        amount: '1000'
      });

      expect(Number(await tokenA.methods.balanceOf(starport._address).call())).toEqual(500);
      expect(Number(await tokenA.methods.balanceOf(account2).call())).toEqual(1000);
    });

    it('should unlock via #invoke', async () => {
      await tokenA.methods.transfer(starport._address, 1500).send({ from: root });

      expect(Number(await tokenA.methods.balanceOf(starport._address).call())).toEqual(1500);
      expect(Number(await tokenA.methods.balanceOf(account2).call())).toEqual(0);

      let unlockNotice = buildNotice(starport.methods.unlock(tokenA._address, 1000, account2));
      let signatures = authorityWallets.map(acct => sign(unlockNotice, acct).signature);

      const tx = await send(starport, 'invoke', [unlockNotice, signatures], { from: account2 });

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenA._address,
        account: account2,
        amount: '1000'
      });

      expect(Number(await tokenA.methods.balanceOf(starport._address).call())).toEqual(500);
      expect(Number(await tokenA.methods.balanceOf(account2).call())).toEqual(1000);
    });

    it('should unlock via hand-coded notice', async () => {
      await tokenA.methods.transfer(starport._address, 1500).send({ from: root });

      const unlockNotice =
        "0x4554483a"                                                       + // b'ETH:'
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraId
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraIndex
        "3030303030303030303030303030303030303030303030303030303030303030" + // Parent Hash
        "8bc39207"                                                         + // "unlock(address,uint256,address)"
        "000000000000000000000000" + tokenA._address.slice(2)              + // Asset
        "0000000000000000000000000000000000000000000000000000000000000001" + // Amount
        "00000000000000000000000000000000000000000000000000000000000000FF"   // Account

      const signatures = signAll(unlockNotice, authorityWallets);
      await send(starport, 'invoke', [unlockNotice, signatures], { from: account2 });

      expect(Number(
        await tokenA.methods.balanceOf("0x00000000000000000000000000000000000000FF").call())).toEqual(1);
    });
  });

  describe('#changeAuthorities', () => {
    // TODO: Check external caller
    // TODO: Check authorities length == 0

    it('should change authorities', async () => {
      const nextAuthorities = nRandomWallets(5).map(acct => acct.address);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      const authoritiesBefore = await call(starport, 'getAuthorities');
      expect(authoritiesBefore).toEqual(authorityAddresses);

      const tx = await send(starport, 'changeAuthorities_', [nextAuthorities]);

      const authoritiesAfter = await call(starport, 'getAuthorities');
      expect(authoritiesAfter).toEqual(nextAuthorities);

      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(tx.events.ChangeAuthorities.returnValues).toMatchObject({
        newAuthorities: nextAuthorities
      });

      // TODO: Must increment era?
      // await expect(send(starport, 'changeAuthorities', [notice, sigs])).rejects.toRevert('revert Admin notice must increment era');
    });

    it('should change authorities via #invoke', async () => {
      const nextAuthorities = nRandomWallets(5).map(acct => acct.address);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      const authoritiesBefore = await call(starport, 'getAuthorities');
      expect(authoritiesBefore).toEqual(authorityAddresses);

      let changeAuthoritiesNotice = buildNotice(starport.methods.changeAuthorities(nextAuthorities));
      let signatures = signAll(changeAuthoritiesNotice, authorityWallets);

      const tx = await send(starport, 'invoke', [changeAuthoritiesNotice, signatures], { from: account1 });

      const authoritiesAfter = await call(starport, 'getAuthorities');
      expect(authoritiesAfter).toEqual(nextAuthorities);

      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(tx.events.ChangeAuthorities.returnValues).toMatchObject({
        newAuthorities: nextAuthorities
      });

      // TODO: Must increment era?
      // await expect(send(starport, 'changeAuthorities', [notice, sigs])).rejects.toRevert('revert Admin notice must increment era');
    });

    it('should change authorities via hand-coded notice', async () => {
      const changeAuthoritiesNotice =
        "0x4554483a"                                                       + // b'ETH:'
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraId
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraIndex
        "3030303030303030303030303030303030303030303030303030303030303030" + // Parent Hash
        "14ee45f2"                                                         + // "changeAuthorities(address[])"
        "0000000000000000000000000000000000000000000000000000000000000020" + // Data Offset
        "0000000000000000000000000000000000000000000000000000000000000001" + // List Length
        "0000000000000000000000002020202020202020202020202020202020202020"   // Amount

      const signatures = signAll(changeAuthoritiesNotice, authorityWallets);
      const tx = await send(starport, 'invoke', [changeAuthoritiesNotice, signatures], { from: account1 });

      expect(tx.events.ChangeAuthorities.returnValues).toMatchObject({
        newAuthorities: ["0x2020202020202020202020202020202020202020"]
      });
    });
  });
});
