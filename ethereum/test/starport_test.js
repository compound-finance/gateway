const { expect } = require("chai");
const ABICoder = require("web3-eth-abi");
const Web3Utils = require('web3-utils');
const {
  bigInt,
  call,
  deploy,
  e18,
  e6,
  fromNow,
  getContractAt,
  getNextContractAddress,
  nRandomWallets,
  nRandomAuthorities,
  replaceByte,
  sign,
  signAll,
  toPrincipal,
  ETH_HEADER,
  ETH_ADDRESS,
} = require('./utils');
require('./matchers_ext');

describe('Starport', () => {
  let proxyAdmin;
  let starportImpl;
  let proxy;
  let starport;
  let cash;
  let tokenA;
  let tokenFee;
  let tokenNS;
  let accounts;
  let defaultFrom, root, account1, account2;

  const authorityWallets = nRandomWallets(3);
  const authorityAddresses = authorityWallets.map(acct => acct.address);

  let eraId;
  let eraIndex;
  let parentHash;

  function hashNotice(notice) {
    if (typeof(notice) !== 'string' || notice.slice(0, 2) !== '0x') {
      throw new Error(`Excepted encoded notice, got: ${JSON.stringify(notice)}`);
    }
    return Web3Utils.keccak256(notice);
  }

  function toBytes32(x) {
    if (!x.startsWith("0x")) {
      x = Web3Utils.asciiToHex(x);
    }

    let padding = 66 - x.length;
    return x.toLowerCase() + [...new Array(padding)].map((i) => "0").join("");
  }

  function buildNotice(call, opts = {}) {
    if (opts.newEra) {
      eraId++;
      eraIndex = 0;
    } else {
      // Set new era index
      eraIndex += 1;
    }

    const eraHeader = ABICoder.encodeParameters(['uint256', 'uint256', 'bytes32'], [opts.eraId || eraId, opts.eraIndex || eraIndex, opts.parentHash || parentHash]);
    const encodedCall = typeof(call) === 'string' ? call : call.data;

    let encoded = `${ETH_HEADER}${eraHeader.slice(2)}${encodedCall.slice(2)}`;

    // Set new parent hash
    parentHash = hashNotice(encoded);

    return encoded;
  }

  // Due the transparent proxy, this is the way to read `implementation` and `admin` when calling not as the admin
  // Note: we could call as the admin, but it's important to know how to read this way, anyway.
  async function getProxyImplementation(ethers, contract) {
    return await ethers.provider.getStorageAt(contract.address, '0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc');
  }

  async function getProxyAdmin(ethers, contract) {
    return await ethers.provider.getStorageAt(contract.address, '0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103');
  }

  let testUnlockNotice;
  let testUnlockNoticeHash;
  let testChangeAuthoritiesNotice;

  beforeEach(async () => {
    accounts = await ethers.getSigners();
    [defaultFrom, root, account1, account2] = accounts;
    proxyAdmin = await deploy(ethers, 'ProxyAdmin', [], { from: root });

    const rootNonce = await ethers.provider.getTransactionCount(root.address);
    const cashAddress = getNextContractAddress(root.address, rootNonce + 4);

    starportImpl = await deploy(ethers, 'StarportHarness', [cashAddress, root.address], { from: root });
    proxy = await deploy(ethers, 'TransparentUpgradeableProxy', [
      starportImpl.address,
      proxyAdmin.address,
      "0x"
    ], { from: root });
    cashImpl = await deploy(ethers, 'CashToken', [proxy.address], { from: root });

    starport = await getContractAt(ethers, 'StarportHarness', proxy.address, root);
    await starport.changeAuthorities(authorityAddresses);
    let cashProxy = await deploy(ethers, 'TransparentUpgradeableProxy', [
      cashImpl.address,
      proxyAdmin.address,
      (await cashImpl.populateTransaction.initialize(0, fromNow(0))).data
    ], { from: root });
    cash = await getContractAt(ethers, 'CashToken', cashProxy.address);
    expect(cash.address).toMatchAddress(cashAddress); // Make sure we counted correctly above
    // Give some 100e6 CASH to account1
    let mintPrincipal = await cash.amountToPrincipal(e6(100));
    await starport.mint_(account1.address, mintPrincipal, { from: root.address });
    tokenA = await deploy(ethers, 'FaucetToken', [e18(100), "tokenA", 18, "TKNA"], { from: root });
    tokenFee = await deploy(ethers, 'FeeToken', [e18(100), "tokenFee", 18, "TFEE"], { from: root });
    tokenNS = await deploy(ethers, 'NonStandardToken', [e18(100), "tokenNS", 18, "TNS"], { from: root });

    eraId = 0;
    eraIndex = 0;
    parentHash = "0x0000000000000000000000000000000000000000000000000000000000000000";
    testUnlockNotice = buildNotice(await starport.populateTransaction.unlock(tokenA.address, 1000, accounts[2].address));
    testUnlockNoticeHash = hashNotice(testUnlockNotice);
    testChangeAuthoritiesNotice = buildNotice(await starport.populateTransaction.changeAuthorities([accounts[2].address, accounts[3].address]));
  });

  describe('Unit Tests', () => {
    it.only('should have correct references', async () => {
      expect(await call(starport, 'cash')).toMatchAddress(cash.address);
      expect(await call(starport, 'admin')).toMatchAddress(root);
      expect(await call(cash, 'admin')).toMatchAddress(starport.address);
    });
  });

  describe('Upgradeable', () => {
    it('should be upgradeable to new logic', async () => {
      await starport.count_().send();
      expect(await starport.counter()).toEqualNumber(1);
      expect(await getProxyImplementation(ethers, starport)).toMatchAddress(starportImpl.address);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      let starportImpl2 = await deploy(ethers, 'StarportHarness2', [cash.address, account1.address], { from: root });
      await proxyAdmin.upgrade(starport.address, starportImpl2.address, { from: root });

      expect(await getProxyImplementation(ethers, starport)).toMatchAddress(starportImpl2.address);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);

      expect(await starport.cash()).toMatchAddress(cash.address);
      expect(await starport.admin()).toEqual(account1);
      expect(await starport.counter()).toEqualNumber(1);

      // We'll need this for new function calls
      starport = await getContractAt(ethers, 'StarportHarness2', proxy.address);
      expect(await starport.mul_(10)).toEqualNumber(10);
    });

    it('should only be upgradeable by the admin', async () => {
      let starportImpl2 = await deploy(ethers, 'StarportHarness2', [cash.address, account1], { from: root });
      await expect(proxyAdmin.upgrade(starport.address, starportImpl2.address, { from: account1 })).rejects.toRevert('revert Ownable: caller is not the owner');
      expect(await getProxyImplementation(ethers, starport)).toMatchAddress(starportImpl.address);
    });

    it('should allow initialization during upgrade', async () => {
      await starport.count_().send();
      expect(await starport.counter()).toEqualNumber(1);
      let starportImpl2 = await deploy(ethers, 'StarportHarness2', [cash.address, account1], { from: root });
      await proxyAdmin.upgradeAndCall(
        starport.address,
        starportImpl2.address,
        starportImpl2.initialize_(10).encodeABI()
      , { from: root });
      expect(await getProxyImplementation(ethers, starport)).toMatchAddress(starportImpl2.address);
      expect(await starport.counter()).toEqualNumber(11);
      starport = await getContractAt(ethers, 'StarportHarness2', proxy.address);
      expect(await starport.mul_(10)).toEqualNumber(110);
    });

    it('should not allow re-initialization during upgrade', async () => {
      let starportImpl2 = await deploy(ethers, 'StarportHarness2', [cash.address, account1], { from: root });
      await proxyAdmin.upgradeAndCall(
        starport.address,
        starportImpl2.address,
        starportImpl2.initialize_(10).encodeABI()
      , { from: root });
      expect(await starport.counter()).toEqualNumber(10);
      await starport.count_().send();
      starport = await getContractAt(ethers, 'StarportHarness2', proxy.address);
      await expect(starport.initialize_(100)).rejects.toRevert("revert cannot reinitialize");
      expect(await starport.counter()).toEqualNumber(11);
    });

    it('should be able to rotate proxy admin', async () => {
      expect(await proxyAdmin.getProxyAdmin(ethers, starport.address)).toMatchAddress(proxyAdmin.address);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      expect(await starport.admin()).toMatchAddress(root);
      await proxyAdmin.changeProxyAdmin(starport.address, account1, { from: root });
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(account1);
      expect(await starport.admin()).toMatchAddress(root);
    });

    it('should not allow rotation unless admin', async () => {
      expect(await proxyAdmin.getProxyAdmin(ethers, starport.address)).toMatchAddress(proxyAdmin.address);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      expect(await starport.admin()).toMatchAddress(root);
      await expect(proxyAdmin.changeProxyAdmin(starport.address, account1, { from: account1 })).rejects.toRevert("revert Ownable: caller is not the owner");
      expect(await proxyAdmin.getProxyAdmin(ethers, starport.address)).toMatchAddress(proxyAdmin.address);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      expect(await starport.admin()).toMatchAddress(root);
    });

    it('should be able to rotate proxy admin\'s admin', async () => {
      expect(await proxyAdmin.owner()).toMatchAddress(root);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      await proxyAdmin.transferOwnership(account1, { from: root });
      expect(await proxyAdmin.owner()).toMatchAddress(account1);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
    });

    it('should not be able to rotate proxy admin\'s admin unless from current admin', async () => {
      expect(await proxyAdmin.owner()).toMatchAddress(root);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
      await expect(proxyAdmin.transferOwnership(account1, { from: account1 })).rejects.toRevert("revert Ownable: caller is not the owner");;
      expect(await proxyAdmin.owner()).toMatchAddress(root);
      expect(await getProxyAdmin(ethers, starport)).toMatchAddress(proxyAdmin.address);
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
    // it.todo('should recover EIP-155 signature');
  });

  describe('#lock', () => {
    it('should lock an asset', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, tokenA.address], { from: account1 });
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenA, 'balanceOf', [starport.address])).toEqualNumber(lockAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenA.address,
        amount: lockAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1)
      });
    });

    it('should fail to lock with insufficient allowance', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, 0], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(10)], { from: root });

      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      await expect(send(starport, 'lock', [e18(1), tokenA.address], { from: account1 })).rejects.toRevert("revert TransferFrom: Inadequate allowance");
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre).toEqualNumber(balancePost);
    });

    it('should fail to lock with insufficient balance', async () => {
      await send(tokenA, "allocateTo", [account1, e18(0)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(10)], { from: root });

      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      await expect(send(starport, 'lock', [e18(1), tokenA.address], { from: account1 })).rejects.toRevert("revert TransferFrom: Inadequate balance");
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre).toEqualNumber(balancePost);
    });

    it.skip('should fail to lock with zero balance', async () => {
      await send(tokenA, "allocateTo", [account1, e18(0)]);
      await send(tokenA, "approve", [starport.address, e18(0)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(10)], { from: root });

      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      await expect(send(starport, 'lock', [e18(0), tokenA.address], { from: account1 })).rejects.toRevert("revert TransferFrom: Inadequate balance");
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre).toEqualNumber(balancePost);
    });

    it('should fail to lock when supply cap exceeded', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(1)], { from: root });

      const lockAmount = e18(2);
      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      await expect(send(starport, 'lock', [lockAmount, tokenA.address], { from: account1 })).rejects.toRevert('revert Supply Cap Exceeded');
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(0);
      expect(await call(tokenA, 'balanceOf', [starport.address])).toEqualNumber(0);
    });

    it('should fail to lock when supply cap exceeded by second lock', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(1)], { from: root });

      const lockAmount = e18(1);
      await send(starport, 'lock', [lockAmount, tokenA.address], { from: account1 });
      await expect(send(starport, 'lock', [lockAmount, tokenA.address], { from: account1 })).rejects.toRevert('revert Supply Cap Exceeded');
    });

    it('should lock a non-standard asset', async () => {
      await send(tokenNS, "transfer", [account1, e18(10)], { from: root });
      await send(tokenNS, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenNS.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const balancePre = bigInt(await call(tokenNS, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, tokenNS.address], { from: account1 });
      const balancePost = bigInt(await call(tokenNS, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenNS, 'balanceOf', [starport.address])).toEqualNumber(lockAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenNS.address,
        amount: lockAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
      });
    });

    it('should lock a fee token', async () => {
      await send(tokenFee, "transfer", [account1, e18(10)], { from: root });
      await send(tokenFee, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenFee.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const lockReceiptAmount = e18(1) / 2n;
      const balancePre = bigInt(await call(tokenFee, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, tokenFee.address], { from: account1 });
      const balancePost = bigInt(await call(tokenFee, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenFee, 'balanceOf', [starport.address])).toEqualNumber(lockReceiptAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenFee.address,
        amount: lockReceiptAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
      });
    });

    // TODO: Fail to lock a fee token when balance after fee is zero

    it('should not calculate supply cap against fee', async () => {
      await send(tokenFee, "transfer", [account1, e18(10)], { from: root });
      await send(tokenFee, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenFee.address, e18(1)], { from: root });

      const lockAmount = e18(2);
      const lockReceiptAmount = e18(2) / 2n;
      const balancePre = bigInt(await call(tokenFee, 'balanceOf', [account1]));
      const tx = await send(starport, 'lock', [lockAmount, tokenFee.address], { from: account1 });
      const balancePost = bigInt(await call(tokenFee, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenFee, 'balanceOf', [starport.address])).toEqualNumber(lockReceiptAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenFee.address,
        amount: lockReceiptAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
      });
    });

    it('should lock cash', async () => {
      const lockAmount = e6(1);
      const balancePre = bigInt(await call(cash, 'balanceOf', [account1]));

      // Approve starport to move tokens first
      await send(cash, 'approve', [starport.address, lockAmount], {from: account1});

      const tx = await send(starport, 'lock', [lockAmount, cash.address], { from: account1 });
      const balancePost = bigInt(await call(cash, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      // Cash is burned, it doesn't live here
      expect(await call(cash, 'balanceOf', [starport.address])).toEqualNumber(0);
      expect(await call(starport, 'cash')).toMatchAddress(cash.address);

      const cashIndex = await call(cash, 'getCashIndex');

      expect(tx.events.LockCash.returnValues).toMatchObject({
        amount: lockAmount.toString(),
        principal: toPrincipal(lockAmount, cashIndex).toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
      });
    });

    it('should fail to lock cash with insufficient balance', async () => {
      const lockAmount = e6(1);
      await send(cash, 'approve', [starport.address, lockAmount], {from: account1});
      const balancePre = bigInt(await call(cash, 'balanceOf', [account1]));
      await send(cash, 'transfer', [starport.address, balancePre], {from: account1}); // Empty the account
      await expect(send(starport, 'lock', [lockAmount, cash.address], { from: account1 })).rejects.toRevert("revert");
      const balancePost = bigInt(await call(cash, 'balanceOf', [account1]));

      expect(balancePost).toEqualNumber(0);
    });

    // it.todo('should fail to lock zero cash');

    it('should not lock eth via lock()', async () => {
      const lockAmount = e18(1);
      const starportEthPre = await ethers.provider.getBalance(starport.address);

      expect(starportEthPre).toEqualNumber(0);

      await expect(call(starport, 'lock', [lockAmount, ETH_ADDRESS], { from: account1 })).rejects.toRevert('revert Please use lockEth');
    });

    it('should lock eth via lockEth()', async () => {
      const lockAmount = e18(1);
      const starportEthPre = await ethers.provider.getBalance(starport.address);
      await send(starport, "setSupplyCap", [ETH_ADDRESS, e18(1)], { from: root });

      expect(starportEthPre).toEqualNumber(0);

      const tx = await send(starport, 'lockEth', [], { from: account1, value: Number(lockAmount) });
      const starportEthPost = await ethers.provider.getBalance(starport.address);

      expect(starportEthPost).toEqualNumber(lockAmount);
      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
        amount: lockAmount.toString()
      });
    });

    // it.todo('should fail to lock zero eth');

    it('should enforce supply cap for Eth', async () => {
      const lockAmount = e18(2);
      const starportEthPre = await ethers.provider.getBalance(starport.address);
      await send(starport, "setSupplyCap", [ETH_ADDRESS, e18(1)], { from: root });

      expect(starportEthPre).toEqualNumber(0);

      await expect(send(starport, 'lockEth', [], { from: account1, value: Number(lockAmount) })).rejects.toRevert('revert Supply Cap Exceeded');
      const starportEthPost = await ethers.provider.getBalance(starport.address);

      expect(starportEthPost).toEqualNumber(0);
    });

    it('fallback lock Eth', async () => {
      const lockAmount = e18(1);
      const starportEthPre = await ethers.provider.getBalance(starport.address);
      await send(starport, "setSupplyCap", [ETH_ADDRESS, e18(1)], { from: root });

      expect(starportEthPre).toEqualNumber(0);

      const tx = await account1.sendTransaction({ to: starport.address, value: Number(lockAmount) });
      const starportEthPost = await ethers.provider.getBalance(starport.address);

      expect(starportEthPost).toEqualNumber(lockAmount);

      let events = await starport.getPastEvents("allEvents", { fromBlock: tx.blockNumber, toBlock: tx.blockNumber })
      expect(events[0].event).toEqual('Lock');
      expect(events[0].returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account1),
        amount: lockAmount.toString()
      });
    });
  });

  describe('#lockTo', () => {
    it('should lock an asset to a recipient', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      const tx = await send(starport, 'lockTo', [lockAmount, tokenA.address, 'ETH', account2.address], { from: account1 });
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenA, 'balanceOf', [starport.address])).toEqualNumber(lockAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenA.address,
        amount: lockAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2),
      });
    });

    it('should fail to lock when supply cap exceeded', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(1)], { from: root });

      const lockAmount = e18(2);
      const balancePre = bigInt(await call(tokenA, 'balanceOf', [account1]));
      await expect(send(starport, 'lockTo', [lockAmount, tokenA.address, 'ETH', account2], { from: account1 })).rejects.toRevert('revert Supply Cap Exceeded');
      const balancePost = bigInt(await call(tokenA, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(0);
      expect(await call(tokenA, 'balanceOf', [starport.address])).toEqualNumber(0);
    });

    it('should fail to lock when supply cap exceeded by second lock', async () => {
      await send(tokenA, "allocateTo", [account1, e18(10)]);
      await send(tokenA, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenA.address, e18(1)], { from: root });

      const lockAmount = e18(1);
      await send(starport, 'lockTo', [lockAmount, tokenA.address, 'ETH', account2], { from: account1 });
      await expect(send(starport, 'lock', [lockAmount, tokenA.address], { from: account1 })).rejects.toRevert('revert Supply Cap Exceeded');
    });

    it('should lock a non-standard asset', async () => {
      await send(tokenNS, "transfer", [account1, e18(10)], { from: root });
      await send(tokenNS, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenNS.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const balancePre = bigInt(await call(tokenNS, 'balanceOf', [account1]));
      const tx = await send(starport, 'lockTo', [lockAmount, tokenNS.address, 'ETH', account2], { from: account1 });
      const balancePost = bigInt(await call(tokenNS, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenNS, 'balanceOf', [starport.address])).toEqualNumber(lockAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenNS.address,
        amount: lockAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2)
      });
    });

    it('should lock a fee token', async () => {
      await send(tokenFee, "transfer", [account1, e18(10)], { from: root });
      await send(tokenFee, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenFee.address, e18(10)], { from: root });

      const lockAmount = e18(1);
      const lockReceiptAmount = e18(1) / 2n;
      const balancePre = bigInt(await call(tokenFee, 'balanceOf', [account1]));
      const tx = await send(starport, 'lockTo', [lockAmount, tokenFee.address, 'ETH', account2], { from: account1 });
      const balancePost = bigInt(await call(tokenFee, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenFee, 'balanceOf', [starport.address])).toEqualNumber(lockReceiptAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenFee.address,
        amount: lockReceiptAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2)
      });
    });

    it('should not calculate supply cap against fee', async () => {
      await send(tokenFee, "transfer", [account1, e18(10)], { from: root });
      await send(tokenFee, "approve", [starport.address, e18(10)], { from: account1 });
      await send(starport, "setSupplyCap", [tokenFee.address, e18(1)], { from: root });

      const lockAmount = e18(2);
      const lockReceiptAmount = e18(2) / 2n;
      const balancePre = bigInt(await call(tokenFee, 'balanceOf', [account1]));
      const tx = await send(starport, 'lockTo', [lockAmount, tokenFee.address, 'ETH', account2], { from: account1 });
      const balancePost = bigInt(await call(tokenFee, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      expect(await call(tokenFee, 'balanceOf', [starport.address])).toEqualNumber(lockReceiptAmount);

      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: tokenFee.address,
        amount: lockReceiptAmount.toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2)
      });
    });

    it('should lock cash', async () => {
      const lockAmount = e6(1);
      const balancePre = bigInt(await call(cash, 'balanceOf', [account1]));

      // Approve starport to move tokens first
      await send(cash, 'approve', [starport.address, lockAmount], {from: account1});

      const tx = await send(starport, 'lockTo', [lockAmount, cash.address, 'ETH', account2], { from: account1 });
      const balancePost = bigInt(await call(cash, 'balanceOf', [account1]));

      expect(balancePre - balancePost).toEqualNumber(lockAmount);
      // Cash is burned, it doesn't live here
      expect(await call(cash, 'balanceOf', [starport.address])).toEqualNumber(0);
      expect(await call(starport, 'cash')).toMatchAddress(cash.address);

      const cashIndex = await call(cash, 'getCashIndex');
      expect(tx.events.LockCash.returnValues).toMatchObject({
        amount: lockAmount.toString(),
        principal: toPrincipal(lockAmount, cashIndex).toString(),
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2)
      });
    });

    it('should not lock eth via lockTo()', async () => {
      const lockAmount = e18(1);
      const starportEthPre = await ethers.provider.getBalance(starport.address);

      expect(starportEthPre).toEqualNumber(0);

      await expect(call(starport, 'lockTo', [lockAmount, ETH_ADDRESS, 'ETH', account2], { from: account1 })).rejects.toRevert('revert Please use lockEth');
    });

    it('should lock eth via lockEthTo()', async () => {
      const lockAmount = e18(1);
      const starportEthPre = await ethers.provider.getBalance(starport.address);
      await send(starport, "setSupplyCap", [ETH_ADDRESS, e18(1)], { from: root });

      expect(starportEthPre).toEqualNumber(0);

      const tx = await send(starport, 'lockEthTo', ['ETH', account2], { from: account1, value: Number(lockAmount) });
      const starportEthPost = await ethers.provider.getBalance(starport.address);

      expect(starportEthPost).toEqualNumber(lockAmount);
      expect(tx.events.Lock.returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        sender: account1,
        chain: 'ETH',
        recipient: toBytes32(account2),
        amount: lockAmount.toString()
      });
    });

    it('should enforce supply cap for Eth', async () => {
      const lockAmount = e18(2);
      const starportEthPre = await ethers.provider.getBalance(starport.address);
      await send(starport, "setSupplyCap", [ETH_ADDRESS, e18(1)], { from: root });

      expect(starportEthPre).toEqualNumber(0);

      await expect(send(starport, 'lockEthTo', ['ETH', account2], { from: account1, value: Number(lockAmount) })).rejects.toRevert('revert Supply Cap Exceeded');
      const starportEthPost = await ethers.provider.getBalance(starport.address);

      expect(starportEthPost).toEqualNumber(0);
    });
  });

  describe('#execTrxRequest', () => {
    it('should emit ExecTrxRequest event', async () => {
      let trxRequest = `(Extract 100 CASH Eth:${account1})`;
      const tx = await send(starport, 'execTrxRequest', [trxRequest], { from: account1 });
      expect(tx.events.ExecTrxRequest.returnValues).toMatchObject({
        account: account1,
        trxRequest
      });
    });
  });

  describe('#executeProposal', () => {
    it('should emit ExecuteProposal event', async () => {
      const extrinsics = ["0x010203", "0x040506"]
      const tx = await send(starport, 'executeProposal', ["My Action", extrinsics], { from: root });
      expect(tx.events.ExecuteProposal.returnValues).toMatchObject({
        title: "My Action",
        extrinsics
      });
    });

    it('should fail if not from admin', async () => {
      const extrinsics = ["0x11", "0x22"]
      await expect(send(starport, 'executeProposal', ["Action", extrinsics], { from: account1 })).rejects.toRevert('revert Call must originate from admin');
    });
  });

  describe('#checkNoticeSignerAuthorized_', () => {
    it('should authorize message', async () => {
      const signatures = signAll(testUnlockNotice, authorityWallets);
      await call(starport, 'checkNoticeSignerAuthorized_', [testUnlockNoticeHash, authorityAddresses, signatures]);
    });

    it('should not authorize duplicate sigs', async () => {
      const duplicateAccounts = Array(3).fill(authorityWallets[0]);
      const signatures = signAll(testUnlockNotice, duplicateAccounts);
      await expect(call(starport, 'checkNoticeSignerAuthorized_', [testUnlockNoticeHash, authorityAddresses, signatures])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should not authorize with too few signatures', async () => {
      const signatures = [sign(testUnlockNotice, authorityWallets[0]).signature];
      await expect(call(starport, 'checkNoticeSignerAuthorized_', [testUnlockNoticeHash, authorityAddresses, signatures])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should not authorize with an unauthorized signer', async () => {
      const badAccounts = nRandomWallets(2);
      const signatures = signAll(testUnlockNotice, badAccounts);
      await expect(call(starport, 'checkNoticeSignerAuthorized_', [testUnlockNoticeHash, authorityAddresses, signatures])).rejects.toRevert('revert Below quorum threshold');
    });
  });

  describe('#invoke', () => {
    it('should invoke simple signed message of current era', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_());
      let signatures = signAll(notice, authorityWallets);

      expect(await call(starport, 'invoke', [notice, signatures])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      let tx = await send(starport, 'invoke', [notice, signatures]);
      expect(tx.events.NoticeInvoked.returnValues).toMatchObject({
        eraId: "0",
        eraIndex: "3",
        noticeHash: hashNotice(notice),
        result: "0x0000000000000000000000000000000000000000000000000000000000000001"
      });
    });

    it('should invoke simple signed message to start next era', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures = signAll(notice, authorityWallets);

      expect(await call(starport, 'invoke', [notice, signatures])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
    });

    it('should correctly increment eras', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures0 = signAll(notice0, authorityWallets);

      expect(await call(starport, 'invoke', [notice0, signatures0])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice0, signatures0]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(1);

      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
      await send(starport, 'invoke', [notice1, signatures1]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(1);

      let notice2 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures2 = signAll(notice2, authorityWallets);

      expect(await call(starport, 'invoke', [notice2, signatures2])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000003'
      );
      await send(starport, 'invoke', [notice2, signatures2]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(2);
    });

    it('should not decrement eras', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures0 = signAll(notice0, authorityWallets);

      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice0, signatures0])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice0, signatures0]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(1);

      let notice2 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures2 = signAll(notice2, authorityWallets);

      expect(await call(starport, 'invoke', [notice2, signatures2])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
      await send(starport, 'invoke', [notice2, signatures2]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(2);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000003'
      );
      await send(starport, 'invoke', [notice1, signatures1]);
      expect(await call(starport, 'eraId', [])).toEqualNumber(2);
    });

    it('should invoke multiple notices', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let signatures0 = signAll(notice0, authorityWallets);

      expect(await call(starport, 'invoke', [notice0, signatures0])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice0, signatures0]);

      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );

      let tx = await send(starport, 'invoke', [notice1, signatures1]);

      expect(tx.events.NoticeInvoked.returnValues).toMatchObject({
        eraId: "0",
        eraIndex: "4",
        noticeHash: hashNotice(notice1),
        result: "0x0000000000000000000000000000000000000000000000000000000000000002"
      });
    });

    it('should not authorize message without current eraId', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_(), { eraId: 1 });
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Notice must use existing era or start next era');
    });

    it('should not authorize message without missing signatures', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_());

      await expect(call(starport, 'invoke', [notice, []])).rejects.toRevert('revert Below quorum threshold');
    });

    it('should fail with invalid magic header [0]', async () => {
      let notice = replaceByte(buildNotice(await starport.populateTransaction.count_()), 0, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[0]');
    });

    it('should fail with invalid magic header [1]', async () => {
      let notice = replaceByte(buildNotice(await starport.populateTransaction.count_()), 1, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[1]');
    });

    it('should fail with invalid magic header [2]', async () => {
      let notice = replaceByte(buildNotice(await starport.populateTransaction.count_()), 2, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[2]');
    });

    it('should fail with invalid magic header [3]', async () => {
      let notice = replaceByte(buildNotice(await starport.populateTransaction.count_()), 3, 'ff');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Invalid header[3]');
    });

    it('should no-op and emit event when passed twice', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_());
      let signatures = signAll(notice, authorityWallets);

      await send(starport, 'invoke', [notice, signatures]);

      expect(await starport.counter()).toEqualNumber(1);

      await expect(await call(starport, 'invoke', [notice, signatures])).toEqual(null)
      let tx = await send(starport, 'invoke', [notice, signatures]);

      expect(tx.events.NoticeReplay.returnValues).toMatchObject({
        noticeHash: hashNotice(notice)
      });

      expect(await starport.counter()).toEqualNumber(1); // Idempotent
    });

    it('should fail with shortened header', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_()).slice(0, 99);
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Must have full header');
    });

    it('should fail with future era id and non-zero index', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_(), { newEra: true, eraIndex: 1 });
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Notice must use existing era or start next era');
    });

    it('should fail with future further era id', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_(), { eraId: 9999, eraIndex: 0 });
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Notice must use existing era or start next era');
    });

    it('should fail with an invalid call', async () => {
      let notice = buildNotice('0x4554483a');
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert Call failed');
    });

    it('should fail with a call which reverts', async () => {
      let notice = buildNotice(await starport.populateTransaction.revert_());
      let signatures = signAll(notice, authorityWallets);

      await expect(call(starport, 'invoke', [notice, signatures])).rejects.toRevert('revert harness reversion');
    });

    it('should pass correct inputs and outputs', async () => {
      let notice = buildNotice(await starport.populateTransaction.math_(5, 6));
      let signatures = signAll(notice, authorityWallets);

      expect(await call(starport, 'invoke', [notice, signatures])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000041'
      );
    });
  });

  describe('#invokeChain', () => {
    it('should invoke for the parent of an accepted notice', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice1, signatures1]);

      expect(await call(starport, 'invokeChain', [notice0, [notice1]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
    });

    it('should chain three notices', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let signatures2 = signAll(notice2, authorityWallets);

      expect(await call(starport, 'invoke', [notice2, signatures2])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice2, signatures2]);

      expect(await call(starport, 'invokeChain', [notice0, [notice1, notice2]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
    });

    it('should chain four notices', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_());
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'invoke', [notice3, signatures3])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice3, signatures3]);

      expect(await call(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
    });

    it('should reject if tail notice not posted', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_());
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'invoke', [notice3, signatures3])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      // Note: skipping `send` here

      await expect(call(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]]))
        .rejects.toRevert('revert Tail notice must have been accepted');
    });

    it('should reject notice with empty notice chain and not previously posted', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_());

      await expect(call(starport, 'invokeChain', [notice, []]))
        .rejects.toRevert('revert Tail notice must have been accepted');
    });

    it('should reject notice with empty notice chain and yes previously posted', async () => {
      let notice = buildNotice(await starport.populateTransaction.count_());
      let signatures = signAll(notice, authorityWallets);

      expect(await call(starport, 'invoke', [notice, signatures])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice, signatures]);

      expect(await starport.counter()).toEqualNumber(1);

      await expect(await call(starport, 'invokeChain', [notice, []])).toEqual(null)
      let tx = await send(starport, 'invokeChain', [notice, []]);

      expect(tx.events.NoticeReplay.returnValues).toMatchObject({
        noticeHash: hashNotice(notice)
      });

      expect(await starport.counter()).toEqualNumber(1); // Idempotent
    });

    it('should chain notices across an era', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      expect(await call(starport, 'invoke', [notice3, signatures3])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice3, signatures3])
      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(await call(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
      await send(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]]);
      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(await call(starport, 'invokeChain', [notice1, [notice2, notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000003'
      );
      await send(starport, 'invokeChain', [notice1, [notice2, notice3]]);
      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(await call(starport, 'invokeChain', [notice2, [notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000004'
      );
      await send(starport, 'invokeChain', [notice2, [notice3]]);
      expect(await call(starport, 'eraId')).toEqualNumber(1);
    });

    it('should chain notices across multiple eras', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures1 = signAll(notice1, authorityWallets);
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_(), { newEra: true });
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice1, signatures1]);
      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(await call(starport, 'invoke', [notice3, signatures3])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
      await send(starport, 'invoke', [notice3, signatures3]);
      expect(await call(starport, 'eraId')).toEqualNumber(2);

      expect(await call(starport, 'invokeChain', [notice2, [notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000003'
      );
      await send(starport, 'invokeChain', [notice2, [notice3]]);
      expect(await call(starport, 'eraId')).toEqualNumber(2);

      // Note: we're using an extended chain here, even though we don't have to
      expect(await call(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000004'
      );
      await send(starport, 'invokeChain', [notice0, [notice1, notice2, notice3]]);
      expect(await call(starport, 'eraId')).toEqualNumber(2);
    });

    it('should reject notice if already accepted', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let signatures1 = signAll(notice1, authorityWallets);

      expect(await call(starport, 'invoke', [notice1, signatures1])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000001'
      );
      await send(starport, 'invoke', [notice1, signatures1]);

      expect(await call(starport, 'invokeChain', [notice0, [notice1]])).toEqual(
        '0x0000000000000000000000000000000000000000000000000000000000000002'
      );
      await send(starport, 'invokeChain', [notice0, [notice1]]);

      // This is this is this
      expect(await starport.counter()).toEqualNumber(2);

      await expect(await call(starport, 'invokeChain', [notice0, [notice1]])).toEqual(null)
      let tx = await send(starport, 'invokeChain', [notice0, [notice1]]);

      expect(tx.events.NoticeReplay.returnValues).toMatchObject({
        noticeHash: hashNotice(notice0)
      });

      expect(await starport.counter()).toEqualNumber(2); // Idempotent
    });

    it('should reject notice if mismatched head notice', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_());
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      await send(starport, 'invoke', [notice3, signatures3])

      await expect(call(starport, 'invokeChain', [notice0, [notice2, notice3]]))
        .rejects.toRevert('revert Notice hash mismatch');
    });

    it('should reject notice if mismatched mid notice', async () => {
      let notice0 = buildNotice(await starport.populateTransaction.count_());
      let notice1 = buildNotice(await starport.populateTransaction.count_());
      let notice2 = buildNotice(await starport.populateTransaction.count_());
      let notice3 = buildNotice(await starport.populateTransaction.count_());
      let signatures3 = signAll(notice3, authorityWallets);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      await send(starport, 'invoke', [notice3, signatures3])

      await expect(call(starport, 'invokeChain', [notice0, [notice1, notice3]]))
        .rejects.toRevert('revert Notice hash mismatch');
    });

    /* Not sure these possible */
    // it.todo('should reject notice if mismatched tail notice');
    // it.todo('consider genesis parent hash of 0x00000000..');
  });

  describe('#unlock', () => {
    it('should unlock asset', async () => {
      await tokenA.transfer(starport.address, 1500, { from: root });

      expect(Number(await tokenA.balanceOf(starport.address))).toEqual(1500);
      expect(Number(await tokenA.balanceOf(account2))).toEqual(0);

      const tx = await send(starport, 'unlock_', [tokenA.address, 1000, account2]);

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenA.address,
        account: account2,
        amount: '1000'
      });

      expect(Number(await tokenA.balanceOf(starport.address))).toEqual(500);
      expect(Number(await tokenA.balanceOf(account2))).toEqual(1000);
    });

    it('should unlock fee token', async () => {
      await tokenFee.transfer(starport.address, 3000, { from: root });

      expect(Number(await tokenFee.balanceOf(starport.address))).toEqual(1500);
      expect(Number(await tokenFee.balanceOf(account2))).toEqual(0);

      const tx = await send(starport, 'unlock_', [tokenFee.address, 1000, account2]);

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenFee.address,
        account: account2,
        amount: '1000' /* I believe this is right (incl fee), but we should agree here that 500 isn't the right number */
      });

      expect(Number(await tokenFee.balanceOf(starport.address))).toEqual(500);
      expect(Number(await tokenFee.balanceOf(account2))).toEqual(500);
    });

    it('should unlock non-standard token', async () => {
      await tokenNS.transfer(starport.address, 1500, { from: root });

      expect(Number(await tokenNS.balanceOf(starport.address))).toEqual(1500);
      expect(Number(await tokenNS.balanceOf(account2))).toEqual(0);

      const tx = await send(starport, 'unlock_', [tokenNS.address, 1000, account2]);

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenNS.address,
        account: account2,
        amount: '1000'
      });

      expect(Number(await tokenNS.balanceOf(starport.address))).toEqual(500);
      expect(Number(await tokenNS.balanceOf(account2))).toEqual(1000);
    });

    it('should not unlock token with insufficient liquidity', async () => {
      await expect(call(starport, 'unlock_', [tokenA.address, 1000, account2])).rejects.toRevert('revert Transfer: insufficient balance');
    });

    it('should unlock eth', async () => {
      const unlockAmount = e18(1);

      await starport.receive_({ from: root, value: Number(unlockAmount) });

      expect(Number(await ethers.provider.getBalance(starport.address))).toEqualNumber(unlockAmount);
      let balancePre = await ethers.provider.getBalance(account2);

      const tx = await send(starport, 'unlock_', [ETH_ADDRESS, unlockAmount, account2]);

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: ETH_ADDRESS,
        account: account2,
        amount: '1000000000000000000'
      });

      expect(Number(await ethers.provider.getBalance(starport.address))).toEqualNumber(0);
      let balancePost = await ethers.provider.getBalance(account2);
      expect(balancePost - balancePre).toEqualNumber(unlockAmount);
    });

    it('should not unlock eth with insufficient liquidity', async () => {
      const unlockAmount = e18(1);

      await expect(call(starport, 'unlock_', [ETH_ADDRESS, unlockAmount, account2])).rejects.toRevert('revert');
    });

    it('should unlock cash', async () => {
      let mintPrincipal = await cash.amountToPrincipal(e6(1));

      expect(Number(await cash.balanceOf(starport.address))).toEqual(0);
      expect(Number(await cash.balanceOf(account2))).toEqual(0);

      const tx = await send(starport, 'unlockCash_', [account2, mintPrincipal]);

      expect(tx.events.UnlockCash.returnValues).toMatchObject({
        account: account2,
        amount: '1000000',
        principal: '1000000'
      });

      expect(Number(await cash.balanceOf(starport.address))).toEqual(0);
      expect(Number(await cash.balanceOf(account2))).toEqualNumber(e6(1));
    });

    it('should unlock via #invoke', async () => {
      await tokenA.transfer(starport.address, 1500, { from: root });

      expect(Number(await tokenA.balanceOf(starport.address))).toEqual(1500);
      expect(Number(await tokenA.balanceOf(account2))).toEqual(0);

      let unlockNotice = buildNotice(await starport.populateTransaction.unlock(tokenA.address, 1000, account2));
      let signatures = authorityWallets.map(acct => sign(unlockNotice, acct).signature);

      const tx = await send(starport, 'invoke', [unlockNotice, signatures], { from: account2 });

      expect(tx.events.Unlock.returnValues).toMatchObject({
        asset: tokenA.address,
        account: account2,
        amount: '1000'
      });

      expect(Number(await tokenA.balanceOf(starport.address))).toEqual(500);
      expect(Number(await tokenA.balanceOf(account2))).toEqual(1000);
    });

    it('should unlock via hand-coded notice', async () => {
      await tokenA.transfer(starport.address, 1500, { from: root });

      const unlockNotice =
        "0x4554483a"                                                       + // b'ETH:'
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraId
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraIndex
        "3030303030303030303030303030303030303030303030303030303030303030" + // Parent Hash
        "8bc39207"                                                         + // "unlock(address,uint256,address)"
        "000000000000000000000000" + tokenA.address.slice(2)              + // Asset
        "0000000000000000000000000000000000000000000000000000000000000001" + // Amount
        "00000000000000000000000000000000000000000000000000000000000000FF"   // Account

      const signatures = signAll(unlockNotice, authorityWallets);
      await send(starport, 'invoke', [unlockNotice, signatures], { from: account2 });

      expect(Number(
        await tokenA.balanceOf("0x00000000000000000000000000000000000000FF"))).toEqual(1);
    });

    it('should fail when not called by self', async () => {
      await expect(call(starport, 'unlock', [tokenA.address, 1000, account1])).rejects.toRevert('revert Call must originate locally');
    });

    it('should fail when insufficient token balance', async () => {
      await expect(call(starport, 'unlock_', [tokenA.address, 1000, account1])).rejects.toRevert('revert Transfer: insufficient balance');
    });
  });

  describe('#changeAuthorities', () => {
    it('should change authorities', async () => {
      const nextAuthorities = nRandomWallets(5).map(acct => acct.address);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      const authoritiesBefore = await call(starport, 'getAuthorities');
      expect(authoritiesBefore).toEqual(authorityAddresses);

      const tx = await send(starport, 'changeAuthorities', [nextAuthorities], { from: root });

      const authoritiesAfter = await call(starport, 'getAuthorities');
      expect(authoritiesAfter).toEqual(nextAuthorities);

      expect(await call(starport, 'eraId')).toEqualNumber(0);

      expect(tx.events.ChangeAuthorities.returnValues).toMatchObject({
        newAuthorities: nextAuthorities
      });
    });

    it('should change authorities via #invoke', async () => {
      const nextAuthorities = nRandomWallets(5).map(acct => acct.address);

      expect(await call(starport, 'eraId')).toEqualNumber(0);
      const authoritiesBefore = await call(starport, 'getAuthorities');
      expect(authoritiesBefore).toEqual(authorityAddresses);

      let changeAuthoritiesNotice = buildNotice(await starport.populateTransaction.changeAuthorities(nextAuthorities), { newEra: true });
      let signatures = signAll(changeAuthoritiesNotice, authorityWallets);

      const tx = await send(starport, 'invoke', [changeAuthoritiesNotice, signatures], { from: account1 });

      const authoritiesAfter = await call(starport, 'getAuthorities');
      expect(authoritiesAfter).toEqual(nextAuthorities);

      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(tx.events.ChangeAuthorities.returnValues).toMatchObject({
        newAuthorities: nextAuthorities
      });
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

    it('should fail when not called by self or admin', async () => {
      await expect(call(starport, 'changeAuthorities', [[]])).rejects.toRevert('revert Call must be by notice or admin');
    });

    it('should fail when no authorities are passed in', async () => {
      await expect(call(starport, 'changeAuthorities', [[]], { from: root })).rejects.toRevert('revert New authority set can not be empty');
    });
  });

  describe('#setSupplyCap', () => {
    it('should set supply cap', async () => {
      expect(await call(starport, 'eraId')).toEqualNumber(0);
      expect(await call(starport, 'supplyCaps', [tokenA.address])).toEqualNumber(0);

      const tx = await send(starport, 'setSupplyCap', [tokenA.address, 500], { from: root });

      expect(await call(starport, 'supplyCaps', [tokenA.address])).toEqualNumber(500);
      expect(await call(starport, 'eraId')).toEqualNumber(0);

      expect(tx.events.NewSupplyCap.returnValues).toMatchObject({
        asset: tokenA.address,
        supplyCap: "500"
      });
    });

    it('should set supply cap via #invoke', async () => {
      expect(await call(starport, 'eraId')).toEqualNumber(0);
      expect(await call(starport, 'supplyCaps', [tokenA.address])).toEqualNumber(0);

      let setSupplyCapNotice = buildNotice(await starport.populateTransaction.setSupplyCap(tokenA.address, 500), { newEra: true });
      let signatures = signAll(setSupplyCapNotice, authorityWallets);

      const tx = await send(starport, 'invoke', [setSupplyCapNotice, signatures], { from: account1 });

      expect(await call(starport, 'supplyCaps', [tokenA.address])).toEqualNumber(500);

      expect(await call(starport, 'eraId')).toEqualNumber(1);

      expect(tx.events.NewSupplyCap.returnValues).toMatchObject({
        asset: tokenA.address,
        supplyCap: "500"
      });
    });

    it('should set supply cap via hand-coded notice', async () => {
      const setSupplyCapNotice =
        "0x4554483a"                                                       + // b'ETH:'
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraId
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraIndex
        "3030303030303030303030303030303030303030303030303030303030303030" + // Parent Hash
        "571f03e5"                                                         + // "setSupplyCap(address,uint256)"
        "000000000000000000000000" + tokenA.address.slice(2)              + // Asset
        "00000000000000000000000000000000000000000000000000000000000001f4"   // Supply Cap

      const signatures = signAll(setSupplyCapNotice, authorityWallets);
      const tx = await send(starport, 'invoke', [setSupplyCapNotice, signatures], { from: account1 });

      expect(tx.events.NewSupplyCap.returnValues).toMatchObject({
        asset: tokenA.address,
        supplyCap: "500"
      });
    });

    it('should fail when not called by self or admin' , async () => {
      await expect(call(starport, 'setSupplyCap', [tokenA.address, 500])).rejects.toRevert('revert Call must be by notice or admin');
    });

    it('should fail when cash token is passed in', async () => {
      await expect(call(starport, 'setSupplyCap', [cash.address, 500], { from: root })).rejects.toRevert('revert Cash does not accept supply cap');
    });
  });

  describe('#setFutureYield', () => {
    it('should set future yield in Cash Token', async () => {
      const nextCashYield = 1200; // 12%
      const nextCashYieldIndex = 1234;
      const nextCashYieldStart = fromNow(60 * 10); // 10m

      expect(await call(cash, 'cashYieldAndIndex')).toMatchObject({
        yield: "0",
        index: "1000000000000000000", // 1e18
      });
      expect(await call(cash, 'nextCashYieldStart')).toEqualNumber(0);
      expect(await call(cash, 'nextCashYieldAndIndex')).toMatchObject({
        yield: "0",
        index: "0",
      });

      const tx = await send(starport, 'setFutureYield', [nextCashYield, nextCashYieldIndex, nextCashYieldStart], { from: root });

      const expectedYieldEvent = {
        nextCashYield: nextCashYield.toString(),
        nextCashYieldIndex: nextCashYieldIndex.toString(),
        nextCashYieldStart: nextCashYieldStart.toString(),
      };
      expect(tx.events.SetFutureYield[0].returnValues).toMatchObject(expectedYieldEvent);
      expect(tx.events.SetFutureYield[1].returnValues).toMatchObject(expectedYieldEvent);

      expect(await call(cash, 'cashYieldAndIndex')).toMatchObject({
        yield: "0",
        index: "1000000000000000000", // 1e18
      });
      expect(await call(cash, 'nextCashYieldStart')).toEqualNumber(nextCashYieldStart);
      expect(await call(cash, 'nextCashYieldAndIndex')).toMatchObject({
        yield: "1200",
        index: "1234",
      });
    });

    it('should set future yield via #invoke', async () => {
      const nextCashYield = 1200; // 12%
      const nextCashYieldIndex = 1234;
      const nextCashYieldStart = fromNow(60 * 10); // 10m

      let setFutureYieldNotice = buildNotice(await starport.populateTransaction.setFutureYield(nextCashYield, nextCashYieldIndex, nextCashYieldStart), { newEra: true });
      let signatures = signAll(setFutureYieldNotice, authorityWallets);

      const tx = await send(starport, 'invoke', [setFutureYieldNotice, signatures], { from: account1 });

      expect(await call(cash, 'cashYieldAndIndex')).toMatchObject({
        yield: "0",
        index: "1000000000000000000", //1e18
      });
      expect(await call(cash, 'nextCashYieldStart')).toEqualNumber(nextCashYieldStart);
      expect(await call(cash, 'nextCashYieldAndIndex')).toMatchObject({
        yield: "1200",
        index: "1234",
      });

      const expectedYieldEvent = {
        nextCashYield: nextCashYield.toString(),
        nextCashYieldIndex: nextCashYieldIndex.toString(),
        nextCashYieldStart: nextCashYieldStart.toString(),
      };
      expect(tx.events.SetFutureYield[0].returnValues).toMatchObject(expectedYieldEvent);
      expect(tx.events.SetFutureYield[1].returnValues).toMatchObject(expectedYieldEvent);
    });

    it('should set future yield via hand-coded notice', async () => {
      const setFutureYieldNotice =
        "0x4554483a"                                                       + // b'ETH:'
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraId
        "0000000000000000000000000000000000000000000000000000000000000000" + // EraIndex
        "3030303030303030303030303030303030303030303030303030303030303030" + // Parent Hash
        "1e9d77d9"                                                         + // "setFutureYield(uint128,uint128,uint256)"
        "00000000000000000000000000000000000000000000000000000000000004b0" + // nextCashYield
        "00000000000000000000000000000000000000000000000000000000000004d2" + // nextCashYieldIndex
        "0000000000000000000000000000000000000000000000000000000062082f07"   // nextCashYieldStart

      const signatures = signAll(setFutureYieldNotice, authorityWallets);
      const tx = await send(starport, 'invoke', [setFutureYieldNotice, signatures], { from: account1 });

      const expectedYieldEvent = {
        nextCashYield: "1200",
        nextCashYieldIndex: "1234",
        nextCashYieldStart: "1644703495",
      }
      expect(tx.events.SetFutureYield[0].returnValues).toMatchObject(expectedYieldEvent);
      expect(tx.events.SetFutureYield[1].returnValues).toMatchObject(expectedYieldEvent);
    });

    it('should fail when not called by self or admin', async () => {
      await expect(call(starport, 'setFutureYield', [1, 2, 3])).rejects.toRevert('revert Call must be by notice or admin');
    });
  });
});
