import path from "path";
import {
  init,
  sendTransaction,
  deployContractByName,
  getTransactionCode,
} from "flow-js-testing/dist";
import { getContractAddress } from "flow-js-testing/dist/utils/contract";
import { getAccountAddress } from "flow-js-testing/dist/utils/account";
import {
  UFix64,
  UInt256,
  UInt8,
  String as fString,
  Array as fArray,
  Address,
} from "@onflow/types";
import { mintFlow } from "flow-js-testing/dist";
import { getScriptCode } from "flow-js-testing/dist/utils/file";
import { executeScript } from "flow-js-testing/dist/utils/interaction";
import { signWithKey } from "./flow.signer";

const basePath = path.resolve(__dirname, "../cadence");
const STARPORT_CONTRACT_NAME = "Starport";

const AUTHORITIES_DATA = [
  {
    publicKey:
      "b82a83577dd93a351d980dc8e55b378480ac552f7e83d66548a14219e10b52a5a7bb4840b50bfdf68df0e27b2a130a6bed4f5b695ad2e8b628be5b51e41aa6b9",
    privateKey:
      "57566b147fb5431230d8ac3ecf0432993b26510bbc82c8c7f6bae56705dfb3ea",
  },
  {
    publicKey:
      "52165a32bd7bf883837a96b9dbec1a004d3a44f43eac2eba2ff9e6940364b64733cd20dd80d66d367de8bcf1875bfb0b2a7c9beb451572c451d6a6882c72677f",
    privateKey:
      "ae732bb18cbb6705e6f63fa4ccfb89afd8169181bb39482d857102b3dca4fe67",
  },
];

async function deployStarport() {
  const deployer = await getAccountAddress(); // random address

  await mintFlow(deployer, "100.000000");

  let result = await deployContractByName({
    name: STARPORT_CONTRACT_NAME,
    to: deployer,
    args: [],
  });
  expect(result.errorMessage).toEqual("");
  return deployer;
}

async function runTransaction(transactionFileName, starportDeployer, args) {
  const addressMap = await getAddressMap();
  const signers = [starportDeployer];
  const code = await getTransactionCode({
    name: transactionFileName,
    addressMap,
  });
  return await sendTransaction({ code, args, signers });
}

async function getAddressMap() {
  const Starport = await getContractAddress(STARPORT_CONTRACT_NAME);
  return {
    Starport,
  };
}

async function prepareForUnlock(userName) {
  const user = await getAccountAddress(userName);
  const Alice = await getAccountAddress("Alice");

  await deployStarport();

  const address = await getAddressMap();

  // Set supply caps
  const newSupplyCap = "1000.0";
  await runTransaction("starport/set_supply_cap_admin", address.Starport, [
    [newSupplyCap, UFix64],
  ]);

  // User deposits Flow token to Starport
  await depositFlowTokens(user, "100.000000");

  const authorities = [AUTHORITIES_DATA[0].publicKey, AUTHORITIES_DATA[1].publicKey];
  await runTransaction("starport/change_authorities_admin", address.Starport, [
    [authorities, fArray(fString)],
  ]);
}

async function getDataFromStarport(scriptName, args = []) {
  const name = scriptName;

  // Generate addressMap from import statements
  const Starport = await getContractAddress(STARPORT_CONTRACT_NAME);

  const addressMap = {
    Starport,
  };

  let code = await getScriptCode({
    name,
    addressMap,
  });

  const value = await executeScript({
    code,
    args,
  });
  return value;
}

async function getLockedBalance() {
  const name = "get_locked_balance";
  return getDataFromStarport(name);
}

async function getAuthorities() {
  const name = "get_authorities";
  return getDataFromStarport(name);
}

async function getEraId() {
  const name = "get_era_id";
  return getDataFromStarport(name);
}

async function getFlowSupplyCap() {
  const name = "get_flow_supply_cap";
  return getDataFromStarport(name);
}

async function getAccountFlowBalance(userAddress) {
  const name = "get_account_flow_balance";
  return getDataFromStarport(name, [[userAddress, Address]]);
}

async function buildUnlockMessage(
  noticeEraId,
  noticeEraIndex,
  parentNoticeHex,
  toAddress,
  amount
) {
  const name = "build_unlock_message";
  return getDataFromStarport(name, [
    [noticeEraId, UInt256],
    [noticeEraIndex, UInt256],
    [parentNoticeHex, fString],
    [toAddress, Address],
    [amount, UFix64],
  ]);
}

async function buildChangeAuthoritiesMessage(
  noticeEraId,
  noticeEraIndex,
  parentNoticeHex,
  newAuthorities
) {
  const name = "build_change_authorities_message";
  return getDataFromStarport(name, [
    [noticeEraId, UInt256],
    [noticeEraIndex, UInt256],
    [parentNoticeHex, fString],
    [newAuthorities, fArray(fString)],
  ]);
}

async function buildSetSupplyCapMessage(
  noticeEraId,
  noticeEraIndex,
  parentNoticeHex,
  supplyCap
) {
  const name = "build_set_supply_cap_message";
  return getDataFromStarport(name, [
    [noticeEraId, UInt256],
    [noticeEraIndex, UInt256],
    [parentNoticeHex, fString],
    [supplyCap, UFix64],
  ]);
}

async function depositFlowTokens(user, amount) {
  // User mints Flow tokens
  await mintFlow(user, amount);

  // User locks their Flow tokens in Starport
  await runTransaction("starport/setup_starport_user", user, []);
  const lockRes = await runTransaction("starport/lock_flow_tokens", user, [
    [amount, UFix64],
  ]);
  return lockRes;
}

describe("Starport Tests", () => {
  beforeAll(() => {
    init(basePath);
  });

  test("# Deploy Starport", async () => {
    await deployStarport();
  });

  test("# Lock tokens", async () => {
    await deployStarport();

    const address = await getAddressMap();

    // Set supply caps
    const newSupplyCap = "1000.0";
    await runTransaction("starport/set_supply_cap_admin", address.Starport, [
      [newSupplyCap, UFix64],
    ]);

    const Alice = await getAccountAddress("Alice");
    const aliceAmount = "100.000000";
    const Bob = await getAccountAddress("Bob");
    const bobAmount = "50.000000";

    // Starport locked Flow balance is 0 at the beginning
    const starportBalanceBefore = await getLockedBalance();
    expect(starportBalanceBefore).toEqual("0.00000000");

    // Alice deposits Flow token to Starport
    const aliceLockRes = await depositFlowTokens(Alice, aliceAmount);

    const aliceLockEvent = aliceLockRes.events[2].data;
    expect(aliceLockEvent.recipient).toEqual(Alice);
    expect(Number(aliceLockEvent.amount)).toEqual(100.0);
    expect(aliceLockEvent.asset).toEqual("FLOW");

    // Check Starport's Flow locked balance after Alice deposit
    const starportBalanceWithAlice = await getLockedBalance();
    expect(Number(starportBalanceWithAlice)).toEqual(100.0);

    // Bob deposits Flow token to Starport
    const bobLockRes = await depositFlowTokens(Bob, bobAmount);

    const bobLockEvent = bobLockRes.events[2].data;
    expect(bobLockEvent.recipient).toEqual(Bob);
    expect(Number(bobLockEvent.amount)).toEqual(50.0);

    // Check Starport's Flow locked balance after Alice deposit
    const starportBalanceWithBob = await getLockedBalance();
    expect(Number(starportBalanceWithBob)).toEqual(150.0);
  });

  test("# Unlock tokens by notice", async () => {
    // Prepare Starport for execting `Unlock` notice
    await prepareForUnlock("Charlie");

    const address = await getAddressMap();

    const noticeEraId = 1;
    const noticeEraIndex = 0;
    const parentHex = "";

    // Unlock tokens to the given address
    const toAddress = "0xf3fcd2c1a78f5eee";
    const amount = "10.0";

    const unlockMessage = await buildUnlockMessage(
      noticeEraId,
      noticeEraIndex,
      parentHex,
      toAddress,
      amount
    );
    // Sign the message with Authority 1 private key
    const signature1 = signWithKey(
      AUTHORITIES_DATA[0].privateKey,
      unlockMessage
    );
    // Sign the message with Authority 2 private key
    const signature2 = signWithKey(
      AUTHORITIES_DATA[1].privateKey,
      unlockMessage
    );
    const signatures = [signature1, signature2];

    const userBalanceBefore = Number(await getAccountFlowBalance(toAddress));
    const unlockRes = await runTransaction(
      "starport/unlock_flow_tokens_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [signatures, fArray(fString)],
        [toAddress, Address],
        [amount, UFix64],
      ]
    );

    const unlockEvent = unlockRes.events[3].data;

    expect(Number(unlockEvent.amount)).toEqual(10.0);
    expect(unlockEvent.account).toEqual(toAddress);
    expect(unlockEvent.asset).toEqual("FLOW");

    const balance = await getLockedBalance();
    expect(Number(balance)).toEqual(90.0);

    const userBalanceAfter = Number(await getAccountFlowBalance(toAddress));
    expect(userBalanceAfter - userBalanceBefore).toEqual(10.0);

    // Check `eraId` after notice was executed
    const eraIdAfter = await getEraId();
    expect(eraIdAfter).toEqual(1);
  });

  test("# Unlock tokens by admin", async () => {
    const Pete = await getAccountAddress("Pete");
    const Anna = await getAccountAddress("Anna");

    await deployStarport();

    const address = await getAddressMap();

    // Set supply caps
    const newSupplyCap = "1000.0";
    await runTransaction("starport/set_supply_cap_admin", address.Starport, [
      [newSupplyCap, UFix64],
    ]);

    // Charlie deposits Flow token to Starport
    await depositFlowTokens(Pete, "100.000000");

    // Unlock tokens to Alice address
    const toAddress = Anna;
    const amount = "10.0";

    const userBalanceBefore = Number(await getAccountFlowBalance(toAddress));
    const unlockRes = await runTransaction(
      "starport/unlock_flow_tokens_admin",
      address.Starport,
      [
        [toAddress, Address],
        [amount, UFix64],
      ]
    );

    const unlockEvent = unlockRes.events[2].data;

    expect(Number(unlockEvent.amount)).toEqual(10.0);
    expect(unlockEvent.account).toEqual(toAddress);
    expect(unlockEvent.asset).toEqual("FLOW");

    const balance = await getLockedBalance();
    expect(Number(balance)).toEqual(90.0);

    const userBalanceAfter = Number(await getAccountFlowBalance(toAddress));
    expect(userBalanceAfter - userBalanceBefore).toEqual(10.0);
  });

  test("# Change authorities by notice", async () => {
    await deployStarport();

    // Check authorities storage field
    const authoritiesBefore = await getAuthorities();
    expect(authoritiesBefore).toEqual([]);

    const address = await getAddressMap();

    const authorities = [
      AUTHORITIES_DATA[0].publicKey,
      AUTHORITIES_DATA[1].publicKey,
    ];
    await runTransaction(
      "starport/change_authorities_admin",
      address.Starport,
      [[authorities, fArray(fString)]]
    );

    // Check authorities storage field
    const authoritiesAdmin = await getAuthorities();
    expect(authoritiesAdmin).toEqual(authorities);

    const noticeEraId = 1;
    const noticeEraIndex = 0;
    const parentHex = "";

    const newAuthorities = [
      "05df808dce3bf02d37990bd76a6e4deaaf5e29ac03677227d42b0d6914403d626256a30fb15d80da9aad7d2b22ffc5a8998043dcf86c38b3d03ea784a33d441a",
      "b97f907e17fcc7cdb98fb8952afbe6c610d78e969336e3577190a10cf4629dc25398d737b67cb0249c8da8bf191ee36686aed9e8172fd90d397d704dc1110ae6",
    ];

    const changeAuthoritiesMessage = await buildChangeAuthoritiesMessage(
      noticeEraId,
      noticeEraIndex,
      parentHex,
      newAuthorities
    );
    // Sign the message with Authority 1 private key
    const signature1 = signWithKey(
      AUTHORITIES_DATA[0].privateKey,
      changeAuthoritiesMessage
    );
    // Sign the message with Authority 2 private key
    const signature2 = signWithKey(
      AUTHORITIES_DATA[1].privateKey,
      changeAuthoritiesMessage
    );
    const signatures = [signature1, signature2];

    const changeAuthoritiesRes = await runTransaction(
      "starport/change_authorities_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [newAuthorities, fArray(fString)],
        [signatures, fArray(fString)],
      ]
    );

    let changeEvent = changeAuthoritiesRes.events[0].data;
    expect(changeEvent.newAuthorities).toEqual(newAuthorities);

    // Check authorities storage field
    const authoritiesAfter = await getAuthorities();
    expect(authoritiesAfter).toEqual(newAuthorities);
  });

  test("# Change authorities by admin", async () => {
    await deployStarport();

    const address = await getAddressMap();

    // Check authorities storage field
    const authoritiesBefore = await getAuthorities();
    expect(authoritiesBefore).toEqual([]);

    const authorities = [
      "6f39d97fbb1a537d154a999636a083e2f85bc6815b7599609eb50d50f534f7773ff29ccf13022ca039edfdb7b0efc79bcc766d5f989c67c009e14a6f0526b6aa",
      "582e62e9a06541e66e7a1033b76e23c70d1520a42c6d7de97548a486942971969964e3e24aae3c88b58e2f4d1213302162b539a5e476d36f63904c82a87a07f2",
    ];
    const authoritiesRes = await runTransaction(
      "starport/change_authorities_admin",
      address.Starport,
      [[authorities, fArray(fString)]]
    );

    const changeEvent = authoritiesRes.events[0].data;

    expect(changeEvent.newAuthorities).toEqual(authorities);

    // Check authorities storage field
    const authoritiesAfter = await getAuthorities();
    expect(authoritiesAfter).toEqual(authorities);
  });

  test("# Set supply cap by notice", async () => {
    await deployStarport();

    const address = await getAddressMap();

    const authorities = [
      AUTHORITIES_DATA[0].publicKey,
      AUTHORITIES_DATA[1].publicKey,
    ];
    await runTransaction(
      "starport/change_authorities_admin",
      address.Starport,
      [[authorities, fArray(fString)]]
    );

    // Check authorities storage field
    const authoritiesAdmin = await getAuthorities();
    expect(authoritiesAdmin).toEqual(authorities);

    const noticeEraId = 1;
    const noticeEraIndex = 0;
    const parentHex = "";
    const newSupplyCap = "1000.0";

    const setSupplyCapMessage = await buildSetSupplyCapMessage(
      noticeEraId,
      noticeEraIndex,
      parentHex,
      newSupplyCap
    );
    // Sign the message with Authority 1 private key
    const signature1 = signWithKey(
      AUTHORITIES_DATA[0].privateKey,
      setSupplyCapMessage
    );
    // Sign the message with Authority 2 private key
    const signature2 = signWithKey(
      AUTHORITIES_DATA[1].privateKey,
      setSupplyCapMessage
    );
    const signatures = [signature1, signature2];

    const setSupplyCapRes = await runTransaction(
      "starport/set_supply_cap_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [newSupplyCap, UFix64],
        [signatures, fArray(fString)],
      ]
    );

    const supplyCapEvent = setSupplyCapRes.events[0].data;

    expect(supplyCapEvent.asset).toEqual("FLOW");
    expect(Number(supplyCapEvent.supplyCap)).toEqual(Number(newSupplyCap));

    const setSupplyCap = await getFlowSupplyCap();
    expect(Number(setSupplyCap)).toEqual(Number(newSupplyCap));
  });

  test("# Set supply cap by admin", async () => {
    await deployStarport();

    const address = await getAddressMap();

    const supplyCap = await getFlowSupplyCap();
    expect(Number(supplyCap)).toEqual(0.0);

    const newSupplyCap = "100.0";
    const supplyCapRes = await runTransaction(
      "starport/set_supply_cap_admin",
      address.Starport,
      [[newSupplyCap, UFix64]]
    );

    const supplyCapEvent = supplyCapRes.events[0].data;
    expect(supplyCapEvent.asset).toEqual("FLOW");
    expect(Number(supplyCapEvent.supplyCap)).toEqual(100.0);

    const setSupplyCap = await getFlowSupplyCap();
    expect(Number(setSupplyCap)).toEqual(Number(newSupplyCap));
  });

  test("# Set supply cap and lock tokens", async () => {
    await deployStarport();

    const address = await getAddressMap();

    const supplyCap = await getFlowSupplyCap();
    expect(Number(supplyCap)).toEqual(0.0);

    const newSupplyCap = "100.0";
    await runTransaction("starport/set_supply_cap_admin", address.Starport, [
      [newSupplyCap, UFix64],
    ]);

    // An attempt to deposit more than supply cap allows
    const Sofia = await getAccountAddress("Sofia");
    const sofiaAmount = "150.000000";

    // Alice deposits Flow token to Starport
    try {
      await depositFlowTokens(Sofia, sofiaAmount);
    } catch (err) {
      expect(err.includes("Supply Cap Exceeded")).toBeTruthy();
    }
    expect.hasAssertions();
  });

  test("# Notice validation error, already invoked", async () => {
    // Prepare Starport for execting `Unlock` notice
    await prepareForUnlock("Scott");

    const address = await getAddressMap();

    const noticeEraId = 1;
    const noticeEraIndex = 0;
    const parentHex = "";
    // Unlock tokens to the given address
    const toAddress = "0xf3fcd2c1a78f5eee";
    const amount = "10.0";

    const unlockMessage = await buildUnlockMessage(
      noticeEraId,
      noticeEraIndex,
      parentHex,
      toAddress,
      amount
    );
    // Sign the message with Authority 1 private key
    const signature1 = signWithKey(
      AUTHORITIES_DATA[0].privateKey,
      unlockMessage
    );
    // Sign the message with Authority 2 private key
    const signature2 = signWithKey(
      AUTHORITIES_DATA[1].privateKey,
      unlockMessage
    );
    const signatures = [signature1, signature2];

    await runTransaction(
      "starport/unlock_flow_tokens_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [signatures, fArray(fString)],
        [toAddress, Address],
        [amount, UFix64],
      ]
    );

    // An attempt to execute the same notice twice
    const unlockErrRes = await runTransaction(
      "starport/unlock_flow_tokens_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [signatures, fArray(fString)],
        [toAddress, Address],
        [amount, UFix64],
      ]
    );
    // Check emitted error
    const unlockErrEvent = unlockErrRes.events[0].data;
    expect(unlockErrEvent.noticeEraId).toEqual(noticeEraId);
    expect(unlockErrEvent.noticeEraIndex).toEqual(noticeEraIndex);
    expect(unlockErrEvent.error).toEqual("Notice replay");
  });

  test("# Notice validation error, invalid signatures", async () => {
    // Prepare Starport for execting `Unlock` notice
    await prepareForUnlock("Adam");

    const address = await getAddressMap();

    const noticeEraId = 1;
    const noticeEraIndex = 0;
    const parentHex = "";
    // Unlock tokens to the given address
    const toAddress = "0xf3fcd2c1a78f5eee";
    const amount = "10.0";
    const signatures = [
      "d8661ebbbe0cc415063a6027fadf6d78b883b88f0ea3b7a15d839b126aa85b55b273e2aeb777a07d04288b432897409e50d4cbadb28f4cf072c5bd3b9220d30e",
      "f01e292593ef04dacc9d35090182b831197fbae7b900585d822b901aa05df75ff505e167a3e2504cc2a0bd928507eaa3e81b9dfbd6878ec9d2e121f0b69537c8",
    ];

    const unlockRes = await runTransaction(
      "starport/unlock_flow_tokens_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [signatures, fArray(fString)],
        [toAddress, Address],
        [amount, UFix64],
      ]
    );

    const unlockErrEvent = unlockRes.events[0].data;
    expect(unlockErrEvent.noticeEraId).toEqual(noticeEraId);
    expect(unlockErrEvent.noticeEraIndex).toEqual(noticeEraIndex);
    expect(unlockErrEvent.error).toEqual("Signatures are incorrect");
  });

  test("# Notice validation error, invalid era", async () => {
    // Prepare Starport for execting `Unlock` notice
    await prepareForUnlock("Emily");

    const address = await getAddressMap();

    const noticeEraId = 3;
    const noticeEraIndex = 0;
    const parentHex = "";
    // Unlock tokens to the given address
    const toAddress = "0x179b6b1cb6755e31";
    const amount = "10.0";

    const unlockMessage = await buildUnlockMessage(
      noticeEraId,
      noticeEraIndex,
      parentHex,
      toAddress,
      amount
    );
    // Sign the message with Authority 1 private key
    const signature1 = signWithKey(
      AUTHORITIES_DATA[0].privateKey,
      unlockMessage
    );
    // Sign the message with Authority 2 private key
    const signature2 = signWithKey(
      AUTHORITIES_DATA[1].privateKey,
      unlockMessage
    );
    const signatures = [signature1, signature2];

    const unlockErrRes = await runTransaction(
      "starport/unlock_flow_tokens_notice",
      address.Starport,
      [
        [noticeEraId, UInt256],
        [noticeEraIndex, UInt256],
        [parentHex, fString],
        [signatures, fArray(fString)],
        [toAddress, Address],
        [amount, UFix64],
      ]
    );

    // Check emitted error
    const unlockErrEvent = unlockErrRes.events[0].data;
    expect(unlockErrEvent.noticeEraId).toEqual(noticeEraId);
    expect(unlockErrEvent.noticeEraIndex).toEqual(noticeEraIndex);
    expect(unlockErrEvent.error).toEqual(
      "Notice must use existing era or start next era"
    );
  });
});
