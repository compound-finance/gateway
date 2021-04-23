methods {
    totalCashPrincipal() returns (uint256) envfree;
	amountToPrincipal(uint256) returns (uint128);
	admin() returns (address) envfree;
	indexBaseUnit() returns (uint256) envfree;
    transfer(address,uint256);
	cashYieldAndIndex() returns (uint128,uint128) envfree;
	nextCashYieldAndIndex() returns (uint128,uint128) envfree;
	balanceOf(address) returns (uint256);
	cashPrincipal(address) returns (uint128) envfree;
	burn(address,uint256) returns (uint128);
	mint(address,uint128) returns (uint256);
}

definition MAX_UINT256() returns uint256 = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
definition MAX_UINT128() returns uint256 = 0xffffffffffffffffffffffffffffffff;
definition MAX_ADDRESS() returns uint256 = 0xffffffffffffffffffffffffffffffffffffffff;
// MAX_UINT128 / 1e18
definition MAX_AMOUNT() returns uint256 = 340282366920938487808;
// 1e18
definition INDEX_BASE_UNIT() returns uint256 = 1000000000000000000;

/** x and y are almost equal; y may be _smaller or larger_ than x up to an amount of epsilon */
definition bounded_error_eq(uint x, uint y, uint epsilon) returns bool = x <= y + epsilon && x + epsilon >= y;
/** x and y are almost equal; y may be _smaller_ than x up to an amount of epsilon */
definition only_slightly_larger_than(uint x, uint y, uint epsilon) returns bool = y <= x && x <= y + epsilon;
/** x and y are almost equal; y may be _larger_ than x up to an amount of epsilon */
definition only_slightly_smaller_than(uint x, uint y, uint epsilon) returns bool = x <= y && y <= x + epsilon;

/**
 * A ghost function that shadows the storage variable totalCashPrincipal
 */
ghost principal() returns uint256;
ghost ghostCalculateIndex(uint128, uint128, uint256, uint256) returns uint128 {
	axiom forall uint128 yield. forall uint128 index. forall uint256 start. forall uint256 timestamp. ghostCalculateIndex(yield, index, start, timestamp) == 1000000000000000000;
}

ghost ghostCalculatePrincipal(uint256, uint128) returns uint128 {
	axiom forall uint256 amount1. forall uint256 amount2. forall uint128 index.
					amount1 >= amount2 => ghostCalculatePrincipal(amount1, index) >= ghostCalculatePrincipal(amount2, index);
	axiom forall uint256 amount. forall uint128 index. ghostCalculatePrincipal(amount, index) <= 0xffffffffffffffffffffffffffffffff;
}

/**
 * We use a hook to define principal as the sum of all entries inside cashPrincipal
 */
hook Sstore cashPrincipal[KEY address _] uint128 balance (uint128 old_balance) STORAGE {
    require old_balance <= principal();
    require balance <= MAX_UINT128();
    require old_balance <= MAX_UINT128();
    havoc principal assuming principal@new() == principal@old() + (balance - old_balance);
    require principal() <= MAX_UINT256();
}

hook Sload uint128 calculatedIndex uninterpIndexCalc[KEY uint128 yield][KEY uint128 index][KEY uint256 start][KEY uint256 timestamp] STORAGE {
	require ghostCalculateIndex(yield, index, start, timestamp) == calculatedIndex;
}

hook Sload uint128 calculatedPrincipal uninterpPrincipalCalc[KEY uint256 amount][KEY uint128 index] STORAGE {
	require ghostCalculatePrincipal(amount, index) == calculatedPrincipal;
}

hook Sstore uninterpPrincipalCalc[KEY uint256 amount][KEY uint128 index] uint128 calculatedPrincipal STORAGE {
	havoc ghostCalculatePrincipal assuming (forall uint256 a. forall uint128 i. (a != amount || i != index) => 
											ghostCalculatePrincipal@new(a, i) == ghostCalculatePrincipal@old(a, i)) &&
									ghostCalculatePrincipal@new(amount, index) == calculatedPrincipal;
}

/**
 * Using the ghost principal (defined to be the sum of entries inside cashPrincipal)
 * we prove the invariant that totalCashPrincipal == principal and so it is also an
 * invariant that totalCashPrincipal the sum of entries inside cashPrincipal
 */
rule integrityOfTotalCashPrincipal(method f) {
    env e;
    calldataarg args;
    require totalCashPrincipal() == principal();
    f(e, args);
    assert totalCashPrincipal() == principal();
}

// sanity check for axioms
rule monotonicityOfAmountToPrincipal(uint256 amount1, uint256 amount2, uint256 ghostPrincipal1, uint256 ghostPrincipal2) {
	env e;
	uint128 principal1 = amountToPrincipal(e, amount1);
	uint128 principal2 = amountToPrincipal(e, amount2);

	assert amount1 >= amount2 => principal1 >= principal2;
}

rule cantTransferMoreThanPrincipal(address account, uint256 amount) {
	env e;
	uint128 principal = amountToPrincipal(e, amount);
	uint128 accountPrincipal = cashPrincipal(e.msg.sender);
	require principal > accountPrincipal;
	bool result = transfer@withrevert(e, account, amount);
	assert lastReverted || !result;
}

/**
 * We have learned at least that the return value of mint doesn't necessarily
 * reflect the amount minted if the principal to mint is less than INDEX_BASE_UNIT
 *
 * @status passing
 */
rule inverseOfMintAndBurn(address a, uint256 p) {
	env e1;
	env e2;
	calldataarg args;

	uint128 indexBase = indexBaseUnit();
	uint128 cashIndex = getCashIndex(e1, args);

	uint256 startBalance = balanceOf(e1, a);
	uint128 startPrincipal = cashPrincipal(a);
	uint256 amountMinted = mint(e1, a, p);
	uint256 principalMinted = amountToPrincipal(e1, amountMinted);
	// DANGER DANGER: HIGH VOLTAGE!
	// pretty big assumption here
	require principalMinted == p;
	uint256 intermediateBalance = balanceOf(e2, a);
	uint256 intermediatePrincipal = cashPrincipal(a);
	uint128 principalBurned = burn(e2, a, amountMinted);
	uint256 endBalance = balanceOf(e2, a);
	uint128 endPrincipal = cashPrincipal(a);

	assert intermediatePrincipal >= startPrincipal;
	assert startPrincipal == endPrincipal;
	assert startBalance == endBalance;
	assert principalBurned == p;
//	assert false;
}

/**
 * An attempt to characterize reasonable preconditions to amountToPrincipal() to help
 * figure out transferStandardPrecondition
 * @status passing
 */
rule amountToPrincipalCharacterizeRevert(uint256 amount) {
	env e;
	require e.msg.value == 0;
	require amount * indexBaseUnit@norevert() < MAX_UINT128();
	amountToPrincipal@withrevert(e, amount);
	assert !lastReverted;
}

/**
 * @ERC20
 * - diff  change   balanceOf(e, to) + value <= MAX_UINT256() 
 * 					becomes (balanceOf(e, to) + value) * indexBaseUnit + (indexBaseUnit - 1) < MAX_UINT128()
 *                      1. type is uint128
 *                      2. underlying principal is uint128 + division truncation with indexBaseUnit conversion
 * - diff  add      require from != to
 * - diff  remove   require to != 0
 * - Will time out if calculateIndex is not summarized
 * preconditions checked - no pause
 * @status passing
 */
transferStandardPrecondition(env e, address to, uint128 value)
description "Transfer failed even though to != 0, value > 0, balances match"
{
	require value > 0;
	address from = e.msg.sender;
	require from != to;

	uint256 fromBalance = balanceOf(e, from);
	uint256 toBalance = balanceOf(e, to);
	uint256 indexBase = indexBaseUnit();
	uint256 truncationDiscrepancy = indexBase - 1;

	bool precondition = fromBalance > value  && (toBalance + value) * indexBase + truncationDiscrepancy < MAX_UINT128();
	require precondition;

	bool result = transfer@withrevert(e, to, value);
	bool transferReverted = lastReverted;

	assert !transferReverted && result;
}

/**
 * @ERC20
 * - diff  change   value <= MAX_UINT256() becomes value < MAX_UINT128()
 *                      1. type is uint128
 *                      2. amountToPrincipal() requires strict inequality
 * - diff  add      require from != to
 * - diff  remove   require to != 0
 * - diff  remove   require from != 0
 * - Will time out if calculateIndex is not summarized
 * @status	passing (check?)
 */
transferFromStandardPrecondition(env e, address from, address to, uint256 value)
description "TransferFrom failed even though to != 0, value > 0, balances match, allowance suffices"
{
	require value > 0;
    require from != to;
	require e.msg.value == 0; // is this an okay precondition?

	address spender = e.msg.sender;
	require spender != 0; // checked in cannotTransferFromWithSpenderZero
	uint256 fromBalance = balanceOf(e, from);
	uint256 toBalance = balanceOf(e, to);
	uint256 indexBase = indexBaseUnit();
	uint256 truncationDiscrepancy = indexBase - 1;
	bool precondition = balanceOf(e, from) >= value && (toBalance + value) * indexBase + truncationDiscrepancy < MAX_UINT128() && allowance(e,from,spender) >= value && from != to;

	require precondition;

	bool result = transferFrom@withrevert(e, from, to, value);
	bool transferReverted = lastReverted;

	assert !transferReverted && result;
}

/**
 * @ERC20
 * - diff	remove	require to != 0
 * @status 	timeout
 */
transferCheckPreconds(env e, address to, uint256 value)
{
	require value != 0;
	
	address from = e.msg.sender;
	bool precondition = balanceOf(e, from) >= value;

	bool result = transfer@withrevert(e, to, value);
	bool transferReverted = lastReverted; // loading transferReverted

	// The transfer function must meet the precondition, or to revert.
	assert !precondition => (transferReverted || !result), "If transfer() precondition does not hold, must either revert or return 0";
}

/**
 * @ERC20
 * @status 	timeout
 * @notice try with principal instead of balance
 */
transferCheckEffects(env e, address to, uint256 value)
{
	require to != 0;
	require value != 0;

	address from = e.msg.sender;
    uint256 origBalanceOfFrom = balanceOf(e, from);
    uint256 origBalanceOfTo = balanceOf(e, to);
	bool result = transfer(e, to, value);
	
	// Start checking the effects
	env e2;
	require e2.block.timestamp >= e.block.timestamp && e2.block.number >= e.block.number; // Checking new balance in new, later environment
	uint256 newBalanceOfTo = sinvoke balanceOf(e, to); // loading new balance of recipient
	uint256 newBalanceOfFrom = sinvoke balanceOf(e, from);

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

/**
 * @ERC20
 * @status	passing
 */
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

/**
 * @ERC20
 * @status	timeout
 */
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

/**
 * @ERC20
 * @status	timeout
 */
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
/**
 * @ERC20
 * @status	passing
 */
unexpectedBalanceChangeExtendedAPI(method f, address targetAddress)
description "Function $f, which is not transferFrom, transfer, mint or burn,
should not change balanceOf of targetAddress=$targetAddress
from $origBalanceOfTarget to $newBalanceOfTarget."
{
    env e;
	uint256 origBalanceOfTarget = sinvoke balanceOf(e, targetAddress);

    calldataarg arg;
    require f.selector != transferFrom(address,address,uint256).selector && f.selector != transfer(address,uint256).selector && f.selector != burn(address,uint256).selector && f.selector != mint(address,uint128).selector;
	env ef;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfTarget = sinvoke balanceOf(e2, targetAddress);

    assert newBalanceOfTarget == origBalanceOfTarget;
}

/**
 * @ERC20
 * @status	passing:	most
 *			violated:	burn, mint
 *			timeout:	transfer
 */
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

/**
 * @ERC20 modified
 * @status	passing:	most
 *			timeout:	burn
 */
senderCanOnlyReduceHerOwnPrincipal( method f, address sender, address other)
description "Sender $sender calling method $f (not transferFrom) should only reduce her own balance and not other's.
But other's $other balance updated from $origBalanceOfOther to $newBalanceOfOther."
{
    env e;
    require other != sender;
	uint256 origBalanceOfOther = sinvoke cashPrincipal(other);

    calldataarg arg;
	env ef;
	require ef.msg.sender == sender;
	require f.selector != transferFrom(address,address,uint256).selector;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfOther = sinvoke cashPrincipal(other);

    assert newBalanceOfOther >= origBalanceOfOther;
}

/**
 * @ERC20
 * @status	passing:	most
 *			violated:	burn, transfer, transferFrom
 *			timeout:	mint
 */
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

/**
 * @ERC20 modified
 * @status	passing:	most
 *			timeout:	burn
 */
senderCanOnlyReduceHerOwnPrincipalUnlessAllowanceAllowsIt( method f, address sender, address other)
description "Sender $sender calling method $f (not transferFrom) should only reduce her own balance and not other's.
But other's $other balance updated from $origBalanceOfOther to $newBalanceOfOther."
{
    env e;
    require other != sender;
	uint256 origBalanceOfOther = sinvoke cashPrincipal(other);

	uint256 origAllowance = amountToPrincipal(e, allowance(e, other, sender));

    calldataarg arg;
	env ef;
	require ef.msg.sender == sender;
	invoke f(ef, arg);

	env e2;
    require e2.block.number >= e.block.number;

	uint256 newBalanceOfOther = sinvoke cashPrincipal(other);

    assert newBalanceOfOther >= origBalanceOfOther || origAllowance >= origBalanceOfOther-newBalanceOfOther;
}

/**
 * @ERC20
 * @status	passing
 */
unexpectedTotalSupplyChange(method f, address targetAddress)
description "Function $f should not change total supply from $origTotalSupply to $newTotalSupply."
{
    env e;
    uint256 origTotalSupply = sinvoke totalSupply(e);

    calldataarg arg;

	require f.selector != burn(address,uint256).selector && f.selector != mint(address,uint128).selector;
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

// Characterizing totalSupply
/**
 * @ERC20
 * @status	not tested
 * @notice	we don't care right?
 */
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

/**
 * @ERC20
 * @status	timeout
 */
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

/**
 * @ERC20
 * @status	timeout
 */
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

/**
 * @notice we don't care right?
 */
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

/**
 * @ERC20
 * @status	passing
 */
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

/**
 * @ERC20
 * @status	timeout
 */
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

/**
 * @ERC20
 * @status	passing
 */
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

// Getters

// Basic mint test
/**
 * @ERC20
 * @status	failing (as expected)
 */
noUnlimitedMintingByOwner2
description "The admin may at some stage fail to mint before reaching MAX_UINT -> contract contains conditions to limit minting."
{
	uint256 MAXINT = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF;
	env e;
	uint256 origTotalSupply = sinvoke totalSupply(e);

	address _admin = sinvoke admin();

	uint256 amount;
	require amount > 0;

	require origTotalSupply + amount <= MAXINT; // it is still possible to increase total supply

	address receiver;
	require sinvoke balanceOf(e,receiver) + amount <= MAXINT;

	env e2;
	require e2.msg.sender == _admin && e2.block.number >= e.block.number;

	invoke mint(e2, receiver, amount);
	bool mintReverted = lastReverted;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	assert newTotalSupply > origTotalSupply;
}

/**
 * @ERC20
 * @status	failing
 * @notice	looks like this stems from an over(under?)flow check using division,
 *			and that division is modeled incorrectly
 */
balanceOfShouldNotRevert
description "balanceOf function may revert"
{
	env e;
	address account;
	require account <= MAX_ADDRESS();

	require e.msg.value == 0;
	invoke balanceOf(e, account);

    assert !lastReverted;
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
	address o = sinvoke admin();

	env e2;
	require e2.msg.sender != o;

	calldataarg arg;
	invoke f(e2,arg);

	assert lastReverted, "$f did not revert even though not called by the admin";
}