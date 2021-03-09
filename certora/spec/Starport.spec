using StarportHarness as starport
using ERC20 as token
using NonStandardERC20 as nonStandardToken
using ExcessiveERC20 as excessiveToken
using CashToken as cashToken

methods {
    starport.eraId() returns (uint256) envfree
    starport.admin() returns (address) envfree
    starport.cash() returns (address) envfree
    starport.supplyCaps(address) returns (uint256) envfree
    starport.authorities(uint256) returns (address) envfree
    starport.isNoticeInvoked(bytes32) returns (bool) envfree
    starport.noticeAlreadyInvoked() returns (bool) envfree
    starport.noticeReturnsEmptyString() returns (bool) envfree
    starport.signaturesAuthorizedPrecondition(uint256,uint256,bytes32) envfree
    starport.checkSignatures(uint256,uint256,bytes32) envfree
    starport.hashChainTargetNotParentPrecond(bytes32) envfree
    starport.checkNoticeChain(bytes32) envfree
    starport.hashChainBrokenLinkPrecond(uint256) envfree
    starport.lockTo(uint,address,address)
    starport.balanceInERC20Asset(address) envfree
    starport.isAssert() returns (bool) envfree
    starport.nowAssert() envfree

    cashToken.totalCashPrincipal() returns (uint256) envfree

    transfer(address,uint256) => DISPATCHER(true)
    transferFrom(address,address,uint256) => DISPATCHER(true)
    balanceOf(address) => DISPATCHER(true)
}

definition MAX_UINT256() returns uint256 = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;

/**
 * @title           Invoke is Idempotent
 * @description     Invoking a notice once or twice will result in the same change in state
 * @status          timeout! (-b 2 and -b 1)
 * @notice          This is too hard a rule it seems, but alreadyInvoked /\ invokeSetsInvoked /\
 *                  invokeChainSetsInvoked gives us a similar guarantee
 */
rule invokeChainIdempotent {
    env e;
    calldataarg args;
    uint256 n;
    uint256 authority;
    address asset;
    bytes32 noticeHash;

    storage init_storage = lastStorage;
    require !starport.isAssert();
    invoke starport.invokeNoticeChain(e, args);
    address authority1 = authorities(authority);
    uint256 supplyCap1 = supplyCaps(asset);
    sinvoke starport.nowAssert();
    bool invoked1 = starport.invokeNoticeChain(e, args);
    uint256 era1 = starport.eraId();

    bool isAssert = sinvoke starport.isAssert() at init_storage;
    invoke starport.invokeNoticeChain(e, args);
    invoke starport.invokeNoticeChain(e, args);
    address authority2 = starport.authorities(authority);
    uint256 supplyCap2 = starport.supplyCaps(asset);
    sinvoke starport.nowAssert();
    bool invoked2 = starport.invokeNoticeChain(e, args);
    uint256 era2 = starport.eraId();

    assert era1 == era2;
    assert authority1 == authority2;
    assert supplyCap1 == supplyCap2;
    assert invoked1 == invoked2;
}

/**
 * @status  timeout! (-b 2 and -b 1)
 */
rule invokeSetsInvoked {
    env e;
    calldataarg args;
    require starport.isAssert() == false;
    sinvoke starport.invokeNotice(e, args);
    sinvoke starport.nowAssert();
    assert starport.invokeNotice(e, args);
}

/**
 * @status  passing
 */
rule invokeChainSetsInvoked {
    env e;
    calldataarg args;
    require starport.isAssert() == false;
    sinvoke starport.invokeNoticeChain(e, args);
    sinvoke starport.nowAssert();
    assert starport.invokeNoticeChain(e, args);
}

/**
 * @title           Already Invoked Notice Doesn't Change state
 * @description     If a notice is already invoked, invokeNoticeInternal does not change
 *                  starport state
 * @notice          I'm not 100% sure cashToken.totalCashPrincipal() is getting the total
 *                  principal of starport.cash()?? @Shelly
 * @notice          DON'T RUN WITH --CACHE
 * @status          violated with assert totalCashPrincipal1 == totalCashPrincipal2
 *                  passing with totalCashPrincipal1 == totalCashPrincipal2 (no cache)
 *                  timeout with asserts about balances
 */
rule alreadyInvoked {
    uint256 n;
    uint256 authority;
    address asset;
    bytes32 noticeHash;
    require cashToken == starport.cash();

    address authority1 = authorities(authority);
    uint256 supplyCap1 = supplyCaps(asset);
    bool invoked1 = isNoticeInvoked(noticeHash);
    uint256 era1 = starport.eraId();
    uint256 totalCashPrincipal1 = cashToken.totalCashPrincipal();
//    uint256 cashBalance1 = starport.balanceInERC20Asset(cashToken);
//    uint256 tokenBalance1 = starport.balanceInERC20Asset(token);
//    uint256 nonStandardTokenBalance1 = starport.balanceInERC20Asset(nonStandardToken);
//    uint256 excessiveTokenBalance1 = starport.balanceInERC20Asset(excessiveToken);

    require starport.noticeAlreadyInvoked();
    bool returnsEmptyString = starport.noticeReturnsEmptyString();

    address authority2 = starport.authorities(authority);
    uint256 supplyCap2 = starport.supplyCaps(asset);
    bool invoked2 = starport.isNoticeInvoked(noticeHash);
    uint256 era2 = starport.eraId();
    uint256 totalCashPrincipal2 = cashToken.totalCashPrincipal();
//    uint256 cashBalance2 = starport.balanceInERC20Asset(cashToken);
//    uint256 tokenBalance2 = starport.balanceInERC20Asset(token);
//    uint256 nonStandardTokenBalance2 = starport.balanceInERC20Asset(nonStandardToken);
//    uint256 excessiveTokenBalance2 = starport.balanceInERC20Asset(excessiveToken);

    assert returnsEmptyString;
    assert era1 == era2;
    assert authority1 == authority2;
    assert supplyCap1 == supplyCap2;
    assert invoked1 == invoked2;
    assert totalCashPrincipal1 == totalCashPrincipal2;
//    assert cashBalance1 == cashBalance2;
//    assert tokenBalance1 == tokenBalance2;
//    assert nonStandardTokenBalance1 == nonStandardTokenBalance2;
//    assert excessiveTokenBalance1 == excessiveTokenBalance2;
}

/**
 * @title Supply Cap Limits Starport Balance in Asset
 * @description checks that lockTo reverts if it would cause the balance of starport
 *              to go above its supply cap
 * @notice this rule and indeed assets rely on proper ERC20 behavior (what if transfering allows overflow?)
 * @status  ?
 */
rule supplyCapLimit(uint amount, address recipient) {
    env e;
    require token != cash();
    require e.msg.sender != starport;
    uint256 startBalance = starport.balanceInERC20Asset(token);
    require startBalance + amount > starport.supplyCaps(token);
    invoke starport.lockTo(e, amount, token, recipient);
    assert lastReverted;
}

/**
 * @title Supply Cap Limits Starport Balance in Asset
 * @description checks that lockTo reverts if it would cause the balance of starport
 *              to go above its supply cap for non-standard ERC20 token
 * @notice this rule and indeed assets rely on proper ERC20 behavior (what if transfering allows overflow?)
 * @status  ?
 */
rule supplyCapLimitNonStandard(uint amount, address recipient) {
    env e;
    require nonStandardToken != cash();
    require e.msg.sender != starport;
    uint256 startBalance = starport.balanceInERC20Asset(nonStandardToken);
    require startBalance + amount > starport.supplyCaps(nonStandardToken);
    invoke starport.lockTo(e, amount, nonStandardToken, recipient);
    assert lastReverted;
}

/**
 * @title Supply Cap Limits Starport Balance in Asset
 * @description checks that lockTo reverts if it would cause the balance of starport
 *              to go above its supply cap for non-standard ERC20 token
 * @notice this rule and indeed assets rely on proper ERC20 behavior (what if transfering allows overflow?)
 * @status  ?
 */
rule supplyCapLimitExcessiveNonStandard(uint amount, address recipient) {
    env e;
    require excessiveToken != cash();
    require e.msg.sender != starport;
    require startBalance + amount > starport.supplyCaps(excessiveToken);
    uint256 startBalance = starport.balanceInERC20Asset(excessiveToken);
    invoke starport.lockTo(e, amount, excessiveToken, recipient);
    assert lastReverted;
}

/**
 * @title Invoke Requires Quorum of Signatures
 * @status  ?
 */
rule checkSignatures(uint256 nSignatures, uint256 nAuthorities, bytes32 noticeHash) {
    require nSignatures > 0;
    require nAuthorities >= nSignatures;
    // I don't trust multiplication
    require nAuthorities < (nSignatures - 1) + (nSignatures - 1) + (nSignatures - 1);
    sinvoke starport.signaturesAuthorizedPrecondition(nSignatures, nAuthorities, noticeHash);
    invoke starport.checkSignatures(nSignatures, nAuthorities, noticeHash);
    assert !lastReverted;
}

/**
 * @title Invoke Requires Quorum of Signatures
 * @status  ?
 */
rule checkSignaturesNotEnoughSignatures(uint256 nSignatures, uint256 nAuthorities, bytes32 noticeHash) {
    require nSignatures > 0;
    require nAuthorities > nSignatures + nSignatures + nSignatures;
    sinvoke starport.signaturesAuthorizedPrecondition(nSignatures, nAuthorities, noticeHash);
    invoke starport.checkSignatures(nSignatures, nAuthorities, noticeHash);
    assert lastReverted;
}

/**
 * @title Invoke Requires Valid Notice Chain
 * @status  ?
 */
rule checkNoticeChainTargetNotHead(bytes32 targetHash) {
    sinvoke starport.hashChainTargetNotParentPrecond(targetHash);
    invoke starport.checkNoticeChain(targetHash);
    assert lastReverted;
}

/**
 * @title Invoke Requires Valid Notice Chain
 * @status  ?
 */
rule checkNoticeChainBrokenLink(uint256 randomLink, bytes32 targetHash) {
    sinvoke starport.hashChainBrokenLinkPrecond(randomLink);
    invoke starport.checkNoticeChain(targetHash);
    assert lastReverted;
}

/**
 * Check that supplyCaps is changed only if msg.sender is starport
 * @status  passing
 */
rule onlyStarportChangeSupplyCap(method f) {
    address token;
    uint256 tokenSupplyCap = starport.supplyCaps(token);
    require e.msg.sender != currentContract && e.msg.sender != starport.admin();
    env e;
    calldataarg args;
    f(e, args);
    assert tokenSupplyCap == starport.supplyCaps(token);
}

/**
 * Provides a "characterization" of which methods are restricted
 * to being called by starport
 */
rule characterizeStarportOnlyMethods(method f) {
    env e;
    calldataarg args;
    require e.msg.sender != currentContract;
    f@withrevert(e, args);
    assert lastReverted;
}

/**
 * Provides a "characterization" of which methods are restricted
 * to being called by starport or admin
 */
rule characterizeStarportOrAdminOnlyMethods(method f) {
    env e;
    calldataarg args;
    require e.msg.sender != currentContract && e.msg.sender != admin();
    f@withrevert(e, args);
    assert lastReverted;
}