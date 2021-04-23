using StarportHarnessOrdering as starport
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
 * forall m, n : N. m > n => (m, 0) > (n, 0)
 * forall m, n : N. n > 0 => (m, 0) < (m, n)
 *
 * forall i, j : N. i <= j && isCurrentEra(j) => wasAccepted((i, 0))
 *
 * forall i, j : EraAndIndex. i <= j && willAcceptNoticeAt(j) => alreadyAcceptedNoticeAt(i)
 * - proof outline
 *  
 *
 * willAcceptNoticeAt(noticeEraId, noticeEraIndex) = noticeEraId <= eraId || noticeEraId == eraId + 1 && noticeEraIndex == 0
 * alreadyAcceptedNoticeAt(noticeEraId, noticeEraIndex) = 
 *
 * (helper)
 * invariant: forall era. era <= currentEra => isNoticeInvoked[(era, 0)]
 * invairant: there is only ever one notice with a given (eraId, eraIndex)
 */
rule eraLemma {
    env e;
    calldataarg args;
    calldataarg args1;
    calldataarg args2;

    uint256 era;
    uint256 eraIndex;
    bytes32 noticeHash;
    uint256 era1;
    uint256 eraIndex1;
    bytes32 noticeHash1;

    require !starport.isAssert();
    era, eraIndex, noticeHash = starport.myNoticeInfo(e, args);
    // get notice info
    era1, eraIndex1, noticeHash1 = starport.partialOrderingCall(e, args1);

    // notice unique by era,index: must be guaranteed by compound chain
    require era == era1 && eraIndex == eraIndex1 <=> noticeHash == noticeHash1;

    require eraIndex == 0;
    require era <= starport.eraId() => starport.isNoticeInvoked(noticeHash);

    sinvoke starport.nowAssert();
    // invoke notice
    invoke starport.partialOrderingCall(e, args1);

    assert era <= starport.eraId() => starport.isNoticeInvoked(noticeHash);
}

definition noticeLt(uint256 era1, uint256 index1, uint256 era2, uint256 index2) returns bool = (era1 <= era2 && index1 == 0 && index2 > 0) || (era1 < era2 && index1 == 0 && index2 == 0);
definition noticeLeq(uint256 era1, uint256 index1, uint256 era2, uint256 index2) returns bool = noticeLt(era1, index1, era2, index2) || era1 == era2 && index1 == index2;
/**
 * forall i, j : EraAndIndex. i <= j && willAcceptNoticeAt(j) => alreadyAcceptedNoticeAt(i)
 *
 * @notice  "address(this).call(calldata_)" must be removed from invokeNoticeInternal in order
 *          for this rule to pass. This is because we overapproximate "a call to any function"
 *          by havocing the entirety of storage (including eraId and isNoticeInvoked). We
 *          currently have no other way to handle unresolved calls. To mitigate this we have
 *          added a rule that checks that only invoke and invokeChain modify eraId and
 *          isNoticeInvoked (and presumably, a notice won't call invoke or invokeChain)
 */
rule noticePartialOrdering {
    env e;
    calldataarg args_i;
    calldataarg args_j;
    calldataarg args_call;
    uint256 era_i;
    uint256 era_j;
    uint256 era_call;
    uint256 eraIndex_i;
    uint256 eraIndex_j;
    uint256 eraIndex_call;
    bytes32 noticeHash_i;
    bytes32 noticeHash_j;
    bytes32 noticeHash_call;

    require !starport.isAssert();

    era_i, eraIndex_i, noticeHash_i = starport.partialOrderingCall(e, args_i);
    era_j, eraIndex_j, noticeHash_j = starport.partialOrderingCall(e, args_j);
    era_call, eraIndex_call, noticeHash_call = starport.partialOrderingCall(e, args_call);

    // notice unique by era,index: must be guaranteed by compound chain
    require (era_i == era_call && eraIndex_i == eraIndex_call) <=> noticeHash_i == noticeHash_call;
    require (era_i == era_j && eraIndex_i == eraIndex_j) <=> noticeHash_i == noticeHash_j;
    require (era_j == era_call && eraIndex_j == eraIndex_call) <=> noticeHash_j == noticeHash_call;

    // eraLemma
    uint256 era_pre = starport.eraId();
    require eraIndex_i == 0 => (era_i <= era_pre => starport.isNoticeInvoked(noticeHash_i));

    sinvoke starport.nowAssert();

    // invoke notice
    invoke starport.partialOrderingCall(e, args_call);

    // eraLemma
    uint256 era_post = starport.eraId();
    assert eraIndex_i == 0 => (era_i <= era_post => starport.isNoticeInvoked(noticeHash_i));

    uint256 returnLength_post;
    returnLength_post, _, _ = starport.partialOrderingCall@withrevert(e, args_j);
    bool willAcceptNoticeAt_j = returnLength_post != 0 && !lastReverted;
    assert (noticeLeq(era_i, eraIndex_i, era_j, eraIndex_j) && willAcceptNoticeAt_j) => starport.isNoticeInvoked(noticeHash_i);
}

/**
 * @title   Only Invoking a Notice Modifies Era or Acceptance
 * @status  passing (failing on expected methods only)
 */
rule onlyInvokeChangesEraOrNoticeAccepted(method f) {
    bytes32 noticeHash_1;
    bytes32 noticeHash_2;
    uint256 eraId = starport.eraId();
    require !starport.isNoticeInvoked(noticeHash_1);
    require starport.isNoticeInvoked(noticeHash_2);

    env e;
    calldataarg args;
    f(e, args);

    assert starport.eraId() == eraId;
    assert !starport.isNoticeInvoked(noticeHash_1);
    assert starport.isNoticeInvoked(noticeHash_2);
}

/*
A hand proof for the rule "noticePartialOrdering"

assume i_i = 0 => e_i <= e => wasAccepted[(e_i, i_i)]
invoke invokeNotice((e_call, i_call));
accepted_j := invokeNotice((e_j, i_j));
assert accepted_j && (e_i, i_i) <= (e_j, i_j) =>  wasAccepted[(e_i, i_i)];


H_1:    i_i = 0 => e_i <= e => wasAccepted[(e_i, i_i)]
wasAccepted': N x N
e': N
wasAccepted'': N x N
e'': N
H_2:    accepted_j = (e_j <= e' || (e_j == e' + 1 && i_j == 0) (from invokeNotice((e_j, i_j)))
H_3:    wasAccepted'' = wasAccepted' U {(e_j, i_j)}
H_4:    accepted_j && (e_i, i_i) <= (e_j, i_J)
H_9:    i_i == 0 || (e_i == e_j && i_i == i_j) by H_4 and definition of <=
---------------------------------------------------------------------------------------------------
wasAccepted''[(e_i, i_i)]

case on invokeNotice((e_call, i_call)):
    1,2:  no change or accepted without new era
        H_6: wasAccepted' = wasAccepted || wasAccepted' z wasAccepted U {(e_call, i_call)}
        H_7: e' = e
        H_13: wasAccepted'' = wasAccepted U {(e_j, i_j)} by H_4 and H_6 || wasAccepted U {(e_call, i_call)} U {(e_j, i_j)}
        -------------------------------------------------------------------------------------------
        case on H_9 (disjunction)
        1:
            H_8: i_i == 0 (left disjunct)
            ---------------------------------------------------------------------------------------
            case on H_4, H_2 (disjunction)
            1:
                H_10: e_j <= e'
                H_11: e_i <= e_j    H_8, H_4 and definition of <=
                H_12: e_i <= e      H_11, H_10, H_7
                -----------------------------------------------------------------------------------
                wasAccepted''[(e_i, i_i)] H_13, H_1, H_8, H_12
            2:
                H_14: (e_j == e' + 1 && i_j == 0)
                -----------------------------------------------------------------------------------
                case on H_4, 2nd conjunct (disjunction of <=):
                1:
                    H_15: (e_i, i_i) == (e_j, i_j)
                    -------------------------------------------------------------------------------
                    wasAccepted''[(e_i, i_i)] H_15, H_13
                2:
                    H_16: (e_i, i_i) < (e_j, i_j)
                    H_17: e_i < e_j                 H_16, H_14 (i_j == 0) and definition of <
                    H_18: e_i <= e'                 H_14, H_17
                    H_19: e_i <= e                  H_18, H_7
                    -------------------------------------------------------------------------------
                    wasAccepted''[(e_i, i_i)] H_13, H_1, H_8, H_19
        2:
            H_20: (e_i, i_i) == (e_j, i_j)
            ---------------------------------------------------------------------------------------
            wasAccepted''[(e_i, i_i)] H_13, H_20
    3:  accepted with new era
        H_21: e_call == e + 1 && i_call == 0
        H_22: e' = e + 1
        H_23: wasAccepted' = wasAccepted U {(e_call, i_call)}
        H_27: wasAccepted'' = wasAccepted U {(e_call, i_call)} U {(e_j, i_j)}
        -------------------------------------------------------------------------------------------
        case on H_9 (disjunction)
        1:
            H_24: i_i == 0 (left disjunct)
            ---------------------------------------------------------------------------------------
            case on H_4, H_2 (disjunction)
            1:
                H_25: e_j <= e'
                H_26: e_i <= e_j    H_24, H_4 and definition of <=
                H_28: e_i <= e      H_26, H_25, H_22
                -----------------------------------------------------------------------------------
                wasAccepted''[(e_i, i_i)] H_27, H_1, H_24, H_28
            2:
                H_29: (e_j == e' + 1 && i_j == 0)
                -----------------------------------------------------------------------------------
                case on H_4, 2nd conjunct (disjunction of <=):
                1:
                    H_30: (e_i, i_i) == (e_j, i_j)
                    -------------------------------------------------------------------------------
                    wasAccepted''[(e_i, i_i)] H_30, H_27
                2:
                    H_31: (e_i, i_i) < (e_j, i_j)
                    H_32: e_i < e_j                 H_31, H_29 (i_j == 0) and definition of <
                    H_33: e_i <= e'                 H_32, H_29
                    H_34: e_i <= e + 1              H_33, H_22
                    -------------------------------------------------------------------------------
                    case on H_34:
                    1:
                        H_35:   e_i == e + 1
                        H_36:   e_i != e_j                      H_32
                        H_37    (e_i, i_i) != (e_j, i_j)        H_36
                        H_39    (e_call, i_call) = (e_i, i_i)   H_24, H_35, H_21
                        ---------------------------------------------------------------------------
                        wasAccepted''[(e_i, i_i)] H_39, H_27
                    2:
                        H_40:   e_i < e + 1
                        H_41:   e_i <= e        H_40
                        ---------------------------------------------------------------------------
                        wasAccepted''[(e_i, i_i)]   H_27, H_1, H_24, H_41

        2:
            H_42: e_i == e_j && i_i == i_j
            ---------------------------------------------------------------------------------------
            wasAccepted''[(e_i, i_i)] H_27, H_42
*/