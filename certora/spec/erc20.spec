// Specification for core ERC20 tokens
methods {
	balanceOf(address) returns uint256 
	transfer(address,uint256) returns bool
	transferFrom(address, address, uint256) returns bool
	approve(address, uint256) returns bool
	allowance(address, address) returns uint256
	totalSupply() returns uint256
	// Extended API - not implemented
	mint(address,uint256) returns bool
	burn(uint256)
	owner() returns address
	paused() returns bool
}

// Preconditions checked - no pause
transferStandardPrecondition(env e, address to, uint256 value)
description "Transfer failed even though to != 0, value > 0, balances match"
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	require to != 0;
	require value > 0;

	address from = e.msg.sender;
	bool precondition = sinvoke balanceOf(e, from) >= value && sinvoke balanceOf(e,to) + value <= MAXINT;

	require precondition;

	bool result = invoke transfer(e, to, value);
	bool transferReverted = lastReverted;

	assert !transferReverted && result;
}

cannotTransferFromZero(env e, address to, uint256 value)
description "TransferFrom succeeded with from=0"
{
	address from = 0;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferReverted = lastReverted;

	assert transferReverted || !result, "Succeeded to transferFrom with from=0";
}


cannotTransferFromWithSpenderZero(env e, address from, address to, uint256 value)
description "TransferFrom succeeded with spender=0"
{
	address spender = e.msg.sender;
	require spender == 0;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferReverted = lastReverted;

	assert transferReverted || !result, "Succeeded to transferFrom with spender=0";
}

transferFromStandardPrecondition(env e, address from, address to, uint256 value)
description "TransferFrom failed even though to != 0, value > 0, balances match, allowance suffices"
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	require to != 0;
	require value > 0;
	require from != 0; // checked in cannotTransferFromZero
	// require e.msg.value == 0; // not necessary because enforced by call to balanceOf.

	address spender = e.msg.sender;
	require spender != 0; // checked in cannotTransferFromWithSpenderZero
	bool precondition = sinvoke balanceOf(e, from) >= value && sinvoke balanceOf(e,to) + value <= MAXINT && sinvoke allowance(e,from,spender) >= value;

	require precondition;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferReverted = lastReverted;

	assert !transferReverted && result;
}

zeroCannotApprove(env e, address spender, uint256 value)
description "Approve succeeded for sender=0"
{
	require e.msg.sender == 0;
	address from = e.msg.sender;
	
	bool result = invoke approve(e, spender, value);
	bool reverted = lastReverted;

	assert reverted || !result, "Approve succeeded for sender=0";
}

cannotApproveNonZeroWhenCurrentlyNonZero(env e, address spender, uint256 value)
description "Approve succeeded even though current allowance is non-zero and value is non-zero"
{
	address from = e.msg.sender;
	require from != 0; // checked in canZeroApprove
	require value != 0;
	uint256 currentAllowance = sinvoke allowance(e,from,spender);
	require currentAllowance != 0;

	bool result = invoke approve(e, spender, value);
	bool reverted = lastReverted; // loading transferReverted

	assert reverted || !result, "Approve succeeded even though value is non-zero and current allowance is non-zero";
}

cannotApproveToZeroSpender(env e, address spender, uint256 value)
description "Approve succeeded even though approved to spender=0"
{
	require spender == 0;
	
	bool result = invoke approve(e, spender, value);
	bool reverted = lastReverted;

	assert reverted || !result, "Approve succeeded for spender=0";
}

approveStandardPrecondition(env e, address spender, uint256 value)
description "Approve failed even though current allowance is 0"
{
	// require e.msg.value == 0; // not necessary because enforced by call to allowance.

	address from = e.msg.sender;
	require from != 0; // checked in zeroCannotApprove
	require spender != 0; // checked in cannotApproveToZeroSpender 
	bool precondition = sinvoke allowance(e,from,spender) == 0;

	require precondition;

	bool result = invoke approve(e, spender, value);
	bool reverted = lastReverted; // loading transferReverted

	assert !reverted && result, "approve failed even though meets precondition";
}



// Preconditions checked - with pause
transferStandardPreconditionWithPause(env e, address to, uint256 value)
description "Transfer failed even though to != 0, value > 0, balances match"
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	require to != 0;
	require value > 0;

	// require e.msg.value == 0; // not necessary because enforced by call to balanceOf.

	address from = e.msg.sender;
	bool precondition = sinvoke balanceOf(e, from) >= value && sinvoke balanceOf(e,to) + value <= MAXINT && !(sinvoke paused(e));

	require precondition;

	bool result = invoke transfer(e, to, value);
	bool transferReverted = lastReverted; // loading transferReverted

	assert !transferReverted && result;
}

transferFromStandardPreconditionWithPause(env e, address from, address to, uint256 value)
description "TransferFrom failed even though to != 0, value > 0, balances match, allowance suffices"
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	require to != 0;
	require value > 0;

	// require e.msg.value == 0; // not necessary because enforced by call to balanceOf.

	address spender = e.msg.sender;
	bool precondition = sinvoke balanceOf(e, from) >= value && sinvoke balanceOf(e,to) + value <= MAXINT && sinvoke allowance(e,from,spender) >= value && !(sinvoke paused(e));

	require precondition;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferReverted = lastReverted; // loading transferReverted

	assert !transferReverted && result;
}


approveStandardPreconditionWithPause(env e, address spender, uint256 value)
description "Approve failed even though current allowance is 0"
{
	// require e.msg.value == 0; // not necessary because enforced by call to allowance.

	address from = e.msg.sender;
	bool precondition = sinvoke allowance(e,from,spender) == 0 && !(sinvoke paused(e));

	require precondition;

	bool result = invoke approve(e, spender, value);
	bool reverted = lastReverted; // loading transferReverted

	assert !reverted && result;
}

/*
isApproveLike(env e, method f, address approvedAddress, uint256 amount, address randomAddress)
good_description "method f is potentially approve-like: updates allowance from sender, but not balances"
{
	env eGetters;
	uint256 originalBalanceOfRandomAddress = sinvoke balanceOf(eGetters, randomAddress);

	sinvoke f(e, approvedAddress, amount);

	uint256 newBalanceOfRandomAddress = sinvoke balanceOf(eGetters, randomAddress);

}
*/

transferCheckPreconds(env e, address to, uint256 value)
{
	require to != 0;
	require value != 0;
	
	address from = e.msg.sender;
	bool precondition = sinvoke balanceOf(e, from) >= value;

	bool result = invoke transfer(e, to, value);
	bool transferReverted = lastReverted; // loading transferReverted

	// The transfer function must meet the precondition, or to revert.
	assert !precondition => (transferReverted || !result), "If transfer() precondition does not hold, must either revert or return 0";
}

transferCheckEffects(env e, address to, uint256 value)
{
	require to != 0;
	require value != 0;

	address from = e.msg.sender;
    uint256 origBalanceOfFrom = sinvoke balanceOf(e, from);
    uint256 origBalanceOfTo = sinvoke balanceOf(e, to);
	bool result = sinvoke transfer(e, to, value);
	
	// Start checking the effects
	env e2;
	require e2.block.timestamp >= e.block.timestamp && e2.block.number >= e.block.number; // Checking new balance in new, later environment
	uint256 newBalanceOfTo = sinvoke balanceOf(e2, to); // loading new balance of recipient
	uint256 newBalanceOfFrom = sinvoke balanceOf(e2,from);

	// Compute the expected new balance.
	uint expectedNewBalanceOfTo;
	uint expectedNewBalanceOfFrom;
	if  (from != to && result) {
		require expectedNewBalanceOfTo == origBalanceOfTo + value;
		require expectedNewBalanceOfFrom == origBalanceOfFrom - value;
	} else {
		require expectedNewBalanceOfTo == origBalanceOfTo;
		require expectedNewBalanceOfFrom == origBalanceOfFrom;
	}
	
	// Effects: new balance of recipient is as expected, and it should also be not less than the original balance
	assert newBalanceOfTo == expectedNewBalanceOfTo && newBalanceOfTo >= origBalanceOfTo, "invalid new balance of to";
	assert newBalanceOfFrom == expectedNewBalanceOfFrom && newBalanceOfFrom <= origBalanceOfFrom, "invalid new balance of from";
}

/*
// Transfer related tests
transferIncorrectUpdateOfBalanceForLegalRecipient(env e, address to, uint256 value)
-- split to other rules

*/

transferForIllegalRecipient
description "Checked implementation permits sending to the 0 address via the transfer function."
good_description "Checked implementation does not permit sending funds to the 0 address via the transfer function."
{
	env e;

	uint256 origBalanceOf0 = sinvoke balanceOf(e, 0);

	uint256 value; 
	require value > 0; // zero tokens sent to 0 are less 'interesting' since there is no real effect

	invoke transfer(e, 0, value);

	uint256 newBalanceOf0 = sinvoke balanceOf(e, 0);

	assert newBalanceOf0 == origBalanceOf0, "Transfer to 0 changed the balance of 0 --> it is allowed";
}

transferWithIllegalValue
description "Checked implementation permits a transfer of zero tokens."
good_description "Checked implementation does not permit a transfer of zero tokens."
{
	env e;
	address to;
	require to != 0;

	require e.msg.value == 0;
	bool res = invoke transfer(e, to, 0);

	assert lastReverted || !res;
}

transferMayThrow
description "Checked implementation for transfer may revert."
good_description "Checked implementation for transfer never reverts."
{
	env e;
	require e.msg.value == 0; // transfer is non payable, so this is the only precondition.

	// calldataarg arg;
	// invoke transfer(e, arg);
	address a;
	uint256 v;
	invoke transfer(e, a, v);

	assert !lastReverted;
}

transferMayReturnFalse
description "Checked implementation for transfer may return false (0) and not due to a revert."
good_description "Checked implementation for transfer always returns `true` when not reverting."
{
	env e;
	calldataarg arg;

	bool ret = sinvoke transfer(e, arg);

	assert ret;
}

transferFromMayReturnFalse
description "Checked implementation for transferFrom may return false (0) and not due to a revert."
good_description "Checked implementation for transferFrom always returns `true` when not reverting."
{
	env e;
	calldataarg arg;

	bool ret = sinvoke transferFrom(e, arg);

	assert ret;
}


transferImpactOthers(env e, address to, uint256 value, address other)
description "Unexpected effect of transfer (sender=${e.msg.sender} to=$to value=${value}):
should not change the balance of $other from $origBalanceOfOther to $newBalanceOfOther."
good_description "An invocation of transfer can potentially only affect the balances of sender and recipient."
{
    require e.msg.sender != other && other != to;

    uint256 origBalanceOfOther = sinvoke balanceOf(e,other);

	invoke transfer(e,to,value);

    env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfOther = sinvoke balanceOf(e2, other);
    assert newBalanceOfOther == origBalanceOfOther;
}

transferUnexpectedBalanceChange(env e, address to, uint256 value)
description "A transfer (sender=${e.msg.sender} to=$to value=${value}) changed balances in an unexpected way.
Original balance of sender $balanceOfFrom updated to $updatedBalanceOfFrom.
Original balance of to $balanceOfTo updated to $updatedBalanceOfTo."
good_description "An invocation of transfer may only increase recipient's balance only and decrease sender's balance."
{
	uint256 balanceOfFrom = sinvoke balanceOf(e, e.msg.sender);
	uint256 balanceOfTo = sinvoke balanceOf(e, to);

	invoke transfer(e,to,value);

	uint256 updatedBalanceOfTo = sinvoke balanceOf(e,to);
	uint256 updatedBalanceOfFrom = sinvoke balanceOf(e,e.msg.sender);

	assert updatedBalanceOfTo >= balanceOfTo && updatedBalanceOfFrom <= balanceOfFrom;
}

transferCappedDebit(env e, address to, uint256 value)
description "A transfer debits more than authorized: debit=$debit, while authorized=$value"
good_description "A transfer cannot debit the sender by more than the passed value."
{
	env e0;
	require e0.block.number == e.block.number;
	uint256 balanceOfFrom = sinvoke balanceOf(e0, e.msg.sender);

	invoke transfer(e,to,value);

	uint256 updatedBalanceOfFrom = sinvoke balanceOf(e0,e.msg.sender);

	uint256 debit = balanceOfFrom-updatedBalanceOfFrom;
	assert debit <= value;
}


// Higher-order tests
unexpectedAllowanceChange(method f, address tokenOwner, address spender)
description "Function $f, which is not transferFrom or approve,
should not change allowance of token owner $tokenOwner to spender $spender
from $origSpenderAllowance to $newSpenderAllowance."
{
    env e;
	uint256 origSpenderAllowance = sinvoke allowance(e, tokenOwner, spender);

    calldataarg arg;
    require f.selector != transferFrom(address,address,uint256).selector && f.selector != approve(address,uint256).selector;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newSpenderAllowance = sinvoke allowance(e2, tokenOwner, spender);
    assert newSpenderAllowance == origSpenderAllowance;
}

unexpectedBalanceChange(method f, address targetAddress)
description "Function $f, which is not transferFrom or transfer,
should not change balanceOf of targetAddress=$targetAddress
from $origBalanceOfTarget to $newBalanceOfTarget."
{
    env e;
    uint256 origBalanceOfTarget = sinvoke balanceOf(e, targetAddress);

    calldataarg arg;
    require f.selector != transferFrom(address,address,uint256).selector && f.selector != transfer(address,uint256).selector;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfTarget = sinvoke balanceOf(e2, targetAddress);

    assert newBalanceOfTarget == origBalanceOfTarget;
}

unexpectedBalanceChangeExtendedAPI(method f, address targetAddress)
description "Function $f, which is not transferFrom, transfer, mint or burn,
should not change balanceOf of targetAddress=$targetAddress
from $origBalanceOfTarget to $newBalanceOfTarget."
{
    env e;
	uint256 origBalanceOfTarget = sinvoke balanceOf(e, targetAddress);

    calldataarg arg;
    require f.selector != transferFrom(address,address,uint256).selector && f.selector != transfer(address,uint256).selector && f.selector != burn(uint256).selector && f.selector != mint(address,uint256).selector;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfTarget = sinvoke balanceOf(e2, targetAddress);

    assert newBalanceOfTarget == origBalanceOfTarget;
}

senderCanOnlyReduceHerOwnBalance( method f, address sender, address other)
description "Sender $sender calling method $f (not transferFrom) should only reduce her own balance and not other's.
But other's $other balance updated from $origBalanceOfOther to $newBalanceOfOther."
{
    env e;
    require other != sender;
	uint256 origBalanceOfOther = sinvoke balanceOf(e, other);

    calldataarg arg;
	env ef;
	require ef.msg.sender == sender;
	require f.selector != transferFrom(address,address,uint256).selector;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfOther = sinvoke balanceOf(e2, other);

    assert newBalanceOfOther >= origBalanceOfOther;
}

senderCanOnlyReduceHerOwnBalanceUnlessAllowanceAllowsIt( method f, address sender, address other)
description "Sender $sender calling method $f (not transferFrom) should only reduce her own balance and not other's.
But other's $other balance updated from $origBalanceOfOther to $newBalanceOfOther."
{
    env e;
    require other != sender;
	uint256 origBalanceOfOther = sinvoke balanceOf(e, other);

	uint256 origAllowance = sinvoke allowance(e, other, sender);

    calldataarg arg;
	env ef;
	require ef.msg.sender == sender;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfOther = sinvoke balanceOf(e2, other);

    assert newBalanceOfOther >= origBalanceOfOther || origAllowance >= origBalanceOfOther-newBalanceOfOther;
}


unexpectedTotalSupplyChange(method f, address targetAddress)
description "Function $f should not change total supply from $origTotalSupply to $newTotalSupply."
{
    env e;
    uint256 origTotalSupply = sinvoke totalSupply(e);

    calldataarg arg;

	//require f != mint && f != burn;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number > e.block.number;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	/* some implementations subtracts balance of address(0) and this will have to be accounted for.
		This particular test assumes that totalSupply is only updated from mint, burn.
	 */
    assert newTotalSupply == origTotalSupply;
}

unexpectedTotalSupplyIncrease(method f, address targetAddress)
{
    env e;
    uint256 origTotalSupply = sinvoke totalSupply(e);

    calldataarg arg;

	//require f != mint && f != burn;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	/* some implementations subtracts balance of address(0) and this will have to be accounted for.
		This particular test assumes that totalSupply is only updated from mint, burn.
	 */
    assert newTotalSupply <= origTotalSupply;
}

// Characterizing totalSupply
totalSupplyDoesNotIncludeBalanceOfZeroCheckWithTransferToZero(uint256 expectedChange)
description "function totalSupply does not include the balance of the zero address:
When transferring $expectedChange tokens to the zero address, totalSupply was deducted that amount"
/*
	If the contract does not allow sending to zero address, there must be some other functionality (like burn) that can be used to check for that behavior.
 */
{
	env e;

	uint256 origTotalSupply = sinvoke totalSupply(e);

	// now, transfer to zero a positive number of tokens, require it to succeed.
	require expectedChange > 0;

	sinvoke transfer(e, 0, expectedChange);

	env e2;
	uint256 newTotalSupply = sinvoke totalSupply(e2);

	assert newTotalSupply != origTotalSupply - expectedChange;
}

// TransferFrom related tests
transferFromWrongBalanceOrAllowanceChanges_0(env e, address from, address to, uint256 value)
description "transferFrom (from = $from, to = $to, by ${e.msg.sender}, value = $value) did not update participants balances and allowances as expected."
{
	address caller = e.msg.sender;
	uint256 origBalanceOfFrom = sinvoke balanceOf(e, from);
    uint256 origBalanceOfTo = sinvoke balanceOf(e, to);
	uint256 origAllowance = sinvoke allowance(e, from, caller);

	bool precondition = origBalanceOfFrom >= value && origAllowance >= value;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferFromReverted = lastReverted;
	// The transferFrom function must meet the precondition, or to revert.
	assert precondition || (transferFromReverted || !result);
}

transferFromWrongBalanceOrAllowanceChanges(env e, address from, address to, uint256 value)
description "transferFrom (from = $from, to = $to, by ${e.msg.sender}, value = $value) did not update participants balances and allowances as expected."
{
	address caller = e.msg.sender;
	uint256 origBalanceOfFrom = sinvoke balanceOf(e, from);
    uint256 origBalanceOfTo = sinvoke balanceOf(e, to);
	uint256 origAllowance = sinvoke allowance(e, from, caller);

	bool precondition = origBalanceOfFrom >= value && origAllowance >= value;

	bool result = invoke transferFrom(e, from, to, value);
	bool transferFromReverted = lastReverted;
	// The transferFrom function must meet the precondition, or to revert.
	//assert precondition || (transferFromReverted || !result);

	env e2;
	require e2.block.number >= e.block.number;
	uint256 newBalanceOfFrom = sinvoke balanceOf(e2, from);
	uint256 newBalanceOfTo = sinvoke balanceOf(e2, to);
	uint256 newAllowance = sinvoke allowance(e2,from, caller);

	bool expectedNewBalanceOfFrom = (((transferFromReverted || !result) || from == to) && newBalanceOfFrom == origBalanceOfFrom)
								|| (!((transferFromReverted || !result) || from == to) && newBalanceOfFrom == origBalanceOfFrom - value);
	bool expectedNewBalanceOfTo = (((transferFromReverted || !result) || from == to) && newBalanceOfTo == origBalanceOfTo)
								|| (!((transferFromReverted || !result) || from == to) && newBalanceOfTo == origBalanceOfTo + value);
	bool expectedNewAllowance = ((transferFromReverted || !result) && newAllowance == origAllowance)
								|| (!(transferFromReverted || !result) && newAllowance == (origAllowance - value));


	bool newFromBalanceChanged = (sinvoke balanceOf(e2, from) == origBalanceOfFrom - value);
	bool newToBalanceChanged = (sinvoke balanceOf(e2, to) == origBalanceOfTo + value);
	bool newAllowanceChanged = (sinvoke allowance(e2, from, caller) == origAllowance - value);

	assert expectedNewBalanceOfFrom && expectedNewBalanceOfTo && expectedNewAllowance;
}

transferFromWithIllegalRecipient
description "transferFrom permits sending to zero address."
{
	env e;
	address from;
	uint256 value;

	uint256 origBalanceOfFrom = sinvoke balanceOf(e, from);
	uint256 origAllowance = sinvoke allowance(e, from, e.msg.sender);

	require origBalanceOfFrom >= value && origAllowance >= value;

	invoke transferFrom(e, from, 0, value);

	assert lastReverted;
}

unexpectedTransferFromImpact(env e, address from, address to, uint256 value, address x, address y, address z)
description "transferFrom sender=${e.msg.sender}, from=$from, to=$to, value=$value - should not impact other allowances (e.g. from y=$y to z=$z), balances (e.g. for x=$x), or total supply"
{
	uint256 origBalanceX    = sinvoke balanceOf(e,x);
	uint256 origAllowanceYZ = sinvoke allowance(e, y, z);
	uint256 origTotalSupply = sinvoke totalSupply(e);

	invoke transferFrom(e, from, to, value);

	env e2;
	require e2.block.number >= e.block.number;

	uint256 newBalanceX    = sinvoke balanceOf(e2, x);
	uint256 newAllowanceYZ = sinvoke allowance(e2, y, z);
	uint256 newTotalSupply = sinvoke totalSupply(e2);

	bool sameBalanceOfX = x == from || x == to || origBalanceX == newBalanceX;
	bool sameAllowanceYZ = (y == from && z == e.msg.sender) || newAllowanceYZ == origAllowanceYZ;
	bool sameTotalSupply = origTotalSupply == newTotalSupply;

	assert sameBalanceOfX && sameAllowanceYZ && sameTotalSupply;
}

transferFromCappedDebit(env e, address from, address to, uint256 value)
description "A transfer debits more than authorized: debit=$debit, while authorized=$value"
{
	env e0;
	require e0.block.number == e.block.number;
	uint256 balanceOfFrom = sinvoke balanceOf(e0, e.msg.sender);

	invoke transferFrom(e,from,to,value);

	uint256 updatedBalanceOfFrom = sinvoke balanceOf(e0,e.msg.sender);

	mathint debit = balanceOfFrom-updatedBalanceOfFrom;
	assert debit <= value;
}


// Approve related tests
unexpectedApproveImpact(env e, address spender, uint256 value, address x, address y, address z)
description "approve by ${e.msg.sender} to spender $spender with value $value should update allowance of sender to spender, and nothing else.
Consider checking balance of x=$x or allowance from y=$y to z=$z."
{
	// Load everything that should not change
	uint256 origBalanceOfX = sinvoke balanceOf(e, x);
	uint256 origAllowanceYZ = sinvoke allowance(e, y, z);
	uint256 origTotalSupply = sinvoke totalSupply(e);

    bool retVal = invoke approve(e,spender, value);
	bool approveInvocationReverted = lastReverted;

	env e2;
	require e2.block.number >= e.block.number;

	// load new allowance that should be updated
	uint256 newAllowanceOfSpender = sinvoke allowance(e2,e.msg.sender,spender);

	// load new values that should stay the same
    uint256 newBalanceOfX = sinvoke balanceOf(e2, x);
	uint256 newAllowanceYZ = sinvoke allowance(e2, y, z);
	uint256 newTotalSupply = sinvoke totalSupply(e2);

	bool correctEffectOnSpender = approveInvocationReverted || !retVal || newAllowanceOfSpender == value;
	bool sameTotalSupply = newTotalSupply == origTotalSupply;
	bool sameBalanceOfX = newBalanceOfX == origBalanceOfX;
	bool sameAllowanceYZ = y == e.msg.sender && z == spender || newAllowanceYZ == origAllowanceYZ;
    assert correctEffectOnSpender && sameTotalSupply && sameBalanceOfX && sameAllowanceYZ;
}

approveMustBeAuthorized(env e, method f, address _owner, address spender)
description "Unallowed approval (increase of allowances) for $a"
{
	calldataarg arg;

	env e0;
	uint256 origAllowance = sinvoke allowance(e0, _owner, spender);

	invoke f(e, arg);

	uint256 newAllowance = sinvoke allowance(e0, _owner, spender);

	assert (newAllowance > origAllowance) => e.msg.sender == _owner;
}

// Getters
balanceOfShouldNotRevert
description "balanceOf function may revert"
{
	env e;
	calldataarg arg;

	require e.msg.value == 0;
	invoke balanceOf(e, arg);

    assert !lastReverted;
}

allowanceShouldNotRevert
description "allowance function may revert"
{
	env e;
	calldataarg arg;

	require e.msg.value == 0;
	invoke allowance(e, arg);

    assert !lastReverted;
}

// Basic mint test
noUnlimitedMintingByOwner
description "The owner may at some stage fail to mint before reaching MAX_UINT -> contract contains conditions to limit minting."
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	env e;
	uint256 origTotalSupply = sinvoke totalSupply(e);

	address _owner = sinvoke owner(e);

	uint256 amount;
	require amount > 0;

	require origTotalSupply + amount <= MAXINT; // it is still possible to increase total supply

	address receiver;

	env e2;
	require e2.msg.sender == _owner && e2.block.number >= e.block.number;

	invoke mint(e2, receiver, amount);
	bool mintReverted = lastReverted;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	assert newTotalSupply > origTotalSupply;
}

noUnlimitedMintingByOwner2
description "The owner may at some stage fail to mint before reaching MAX_UINT -> contract contains conditions to limit minting."
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	env e;
	uint256 origTotalSupply = sinvoke totalSupply(e);

	address _owner = sinvoke owner(e);

	uint256 amount;
	require amount > 0;

	require origTotalSupply + amount <= MAXINT; // it is still possible to increase total supply

	address receiver;
	require sinvoke balanceOf(e,receiver) + amount <= MAXINT;

	env e2;
	require e2.msg.sender == _owner && e2.block.number >= e.block.number;

	invoke mint(e2, receiver, amount);
	bool mintReverted = lastReverted;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	assert newTotalSupply > origTotalSupply;
}

// Check for privileged operations
privilegedOperation(method f, address privileged)
description "$f can be called by more than one user without reverting"
{
	env e1;
	calldataarg arg;
	require (e1.msg.sender == privileged);
	invoke f(e1, arg); // privileged succeeds executing candidate privileged operation.
	bool firstSucceeded = !lastReverted;

	env e2;
	calldataarg arg2;
	require (e2.msg.sender != privileged);
	invoke f(e2, arg2); // unprivileged
	bool secondSucceeded = !lastReverted;

	assert !(firstSucceeded && secondSucceeded), "$f can be called by both ${e1.msg.sender} and ${e2.msg.sender}, so it is not privileged";
}

smartPrivilegedOperation(method f, address privileged)
description "$f can be called by more than one user without reverting"
{
	env e1;
	calldataarg arg;
	require (e1.msg.sender == privileged);
	storage initialStorage = lastStorage;
	invoke f(e1, arg); // privileged succeeds executing candidate privileged operation.
	bool firstSucceeded = !lastReverted;

	env e2;
	calldataarg arg2;
	require (e2.msg.sender != privileged);
	invoke f(e2, arg2) at initialStorage; // unprivileged
	bool secondSucceeded = !lastReverted;

	assert !(firstSucceeded && secondSucceeded), "$f can be called by both ${e1.msg.sender} and ${e2.msg.sender}, so it is not privileged";
}

simplePrivilegedOperation(method f)
description "Function $f is not privileged"
{
	env e;
	address o = sinvoke owner(e);

	env e2;
	require e2.msg.sender != o;

	calldataarg arg;
	invoke f(e2,arg);

	assert lastReverted, "$f did not revert even though not called by the owner";
}

// Standard methods implemented test
implementsStandard(env e)
description "If compiles, then checked implementation implements all standard ERC20 functions"
{
	address x;
	address y;
	uint256 a;

	invoke balanceOf(e,x);
	invoke transfer(e,x,a);
	invoke transferFrom(e,x,y,a);
	invoke approve(e,x,a);
	invoke allowance(e,x,y);
	invoke totalSupply(e);

	assert true;
}

hasMint(env e, address argForMint1, uint256 argForMint2)
description "The call to mint({$argForMint1,${argForMint2}) is possible"
{
	invoke mint(e,argForMint1,argForMint2);

	assert true;
}

hasBurn(uint256 argForBurn)
description "The call to burn($argForBurn) is possible"
{
	env e;
	invoke burn(e,argForBurn);

	assert true;
}

hasOwner(env e)
description "There is an explicit owner by name in the contract"
{
	invoke owner(e);
	assert true;
}
