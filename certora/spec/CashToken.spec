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

/**
 * @title Monotonicity of Amount to Principal
 * @status	timing out: this is just too hard for SMT
 */
rule monotonicityOfAmountToPrincipal(uint256 amount1, uint256 amount2) {
	env e;
	uint128 principal1 = amountToPrincipal(e, amount1);
	uint128 principal2 = amountToPrincipal(e, amount2);

	assert amount1 >= amount2 => principal1 >= principal2;
}

/**
 * We have learned at least that the return value of mint doesn't necessarily
 * reflect the amount minted if the principal to mint is less than INDEX_BASE_UNIT
 *
 * @status	timeout
 * @notice	passing in harness
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
 * Checks that if the sender is not starport, that none of
 * cashYieldAndIndex and nextCashYieldAndIndex are characterizeModifyIndex
 *
 * @notice seems like there's no reason not to put an admin check on initialize
 * @status passing except initialize
 */
rule onlyStarportModifyYieldAndIndex(method f) {
    address starport = admin();

	env e;
	calldataarg args;
	require e.msg.sender != starport;
	
	uint128 startYield;
	uint128 startIndex;
	uint128 startNextYield;
	uint128 startNextIndex;
	startYield, startIndex = cashYieldAndIndex();
	startNextYield, startNextIndex = nextCashYieldAndIndex();

	f(e, args);

	uint128 endYield;
	uint128 endIndex;
	uint128 endNextYield;
	uint128 endNextIndex;
	endYield, endIndex = cashYieldAndIndex();
	endNextYield, endNextIndex = nextCashYieldAndIndex();

	assert startYield == endYield &&
				startIndex == endIndex &&
				startNextYield == endNextYield &&
				startNextIndex == endNextIndex;
}

/**
 * @ERC20
 * - looking at the code, I would expect this rule to fail
 *   is this expected behavior?
 * - diff,	REMOVE:		require from != 0
 * @status failing
 */
cannotApproveNonZeroWhenCurrentlyNonZero(env e, address spender, uint256 value)
description "Approve succeeded even though current allowance is non-zero and value is non-zero"
{
	address from = e.msg.sender;
	require value != 0;
	uint256 currentAllowance = allowance(e,from,spender);
	require currentAllowance != 0;

	bool result = approve@withrevert(e, spender, value);
	bool reverted = lastReverted; // loading transferReverted

	assert reverted || !result, "Approve succeeded even though value is non-zero and current allowance is non-zero";
}

/**
 * @ERC20
 * @status passing
 */
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

/**
 * @ERC20
 * - diff	remove	require to != 0
 * @notice try with principal instead of balance
 * @notice	timeout with harness
 */
transferCheckPreconds(env e, address to, uint256 value)
{
	require to != 0;
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
 * @status violated here but timeout in harness
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

/**
 * @notice	we don't care right?
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

/**
 * @ERC20
 * @status	timeout
 * @notice	passing in harness
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
 * @status	passing
 */
unexpectedAllowanceChange(method f, address tokenOwner, address spender)
description "Function $f, which is not transferFrom or approve,
should not change allowance of token admin $tokenOwner to spender $spender
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

/**
 * @ERC20
 * @status	last run failed
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

/**
 * @ERC20
 * @status	timeout
 * @notice	passing in harness
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

/**
 * @ERC20
 * @status	passing
 */
approveMustBeAuthorized(env e, method f, address _admin, address spender)
description "Unallowed approval (increase of allowances) for $a"
{
	calldataarg arg;

	env e0;
	uint256 origAllowance = sinvoke allowance(e0, _admin, spender);

	invoke f(e, arg);

	uint256 newAllowance = sinvoke allowance(e0, _admin, spender);

	assert (newAllowance > origAllowance) => e.msg.sender == _admin;
}

// Basic mint test
/**
 * @ERC20
 * @status	failing (as expected)
 */
noUnlimitedMintingByOwner
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

	env e2;
	require e2.msg.sender == _admin && e2.block.number >= e.block.number;

	invoke mint(e2, receiver, amount);
	bool mintReverted = lastReverted;

	uint256 newTotalSupply = sinvoke totalSupply(e2);

	assert newTotalSupply > origTotalSupply;
}

/**
 * @ERC20
 * @status	timeout
 * @notice	failing (as expected) in harness
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

/***********************
 *  Characterizations  *
 ***********************/

/**
 * @ERC20
 * @status	failing
 */
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

/**
 * @ERC20
 * @status	failing (as expected): transfer may revert
 */
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

/**
 * @ERC20
 * @status	passing: transfer will never return false
 */
transferMayReturnFalse
description "Checked implementation for transfer may return false (0) and not due to a revert."
good_description "Checked implementation for transfer always returns `true` when not reverting."
{
	env e;
	calldataarg arg;

	bool ret = sinvoke transfer(e, arg);

	assert ret;
}

/**
 * @ERC20
 * @status	passing: transferFrom will never return false
 */
transferFromMayReturnFalse
description "Checked implementation for transferFrom may return false (0) and not due to a revert."
good_description "Checked implementation for transferFrom always returns `true` when not reverting."
{
	env e;
	calldataarg arg;

	bool ret = sinvoke transferFrom(e, arg);

	assert ret;
}

/**
 * @ERC20
 * @status	failing: balanceOf may revert
 * @notice	also failing in harness, looks like an uninterp division problem
 */
balanceOfShouldNotRevert
description "balanceOf function may revert"
{
	env e;
	calldataarg arg;

	require e.msg.value == 0;
	invoke balanceOf(e, arg);

    assert !lastReverted;
}

/**
 * @ERC20
 * @status	passing
 */
allowanceShouldNotRevert
description "allowance function may revert"
{
	env e;
	address owner;
	address spender;

	require e.msg.value == 0;
	invoke allowance(e, owner, spender);

    assert !lastReverted;
}

/**
 * Provides a "characterization" of which methods modify the yield
 * field of cashYieldAndIndex
 */
rule characterizeModifyYield(method f) {
    address starport = admin();

	env e;
	calldataarg args;
	require e.msg.sender == starport;

	uint128 startYield;
	startYield, _ = cashYieldAndIndex();
	f(e, args);
	uint128 endYield;
	endYield, _ = cashYieldAndIndex();
	assert startYield == endYield;
}

/**
 * Provides a "characterization" of which methods modify the index
 * field of cashYieldAndIndex
 */
rule characterizeModifyIndex(method f) {
    address starport = admin();
	env e;
	calldataarg args;
	require e.msg.sender == starport;

	uint128 startIndex;
	_, startIndex = cashYieldAndIndex();

	f(e, args);

	uint128 endIndex;
	_, endIndex = cashYieldAndIndex();
	assert startIndex == endIndex;
}

/**
 * Provides a "characterization" of which methods modify the yield
 * field of nextCashYieldAndIndex
 */
rule characterizeModifyNextYield(method f) {
    address starport = admin();
	env e;
	calldataarg args;
	require e.msg.sender == starport;

	uint128 startNextYield;
	startNextYield, _ = nextCashYieldAndIndex();

	f(e, args);

	uint128 endNextYield;
	endNextYield, _ = nextCashYieldAndIndex();

	assert startNextYield == endNextYield;
}

/**
 * Provides a "characterization" of which methods modify the index
 * field of nextCashYieldAndIndex
 */
rule characterizeModifyNextIndex(method f) {
    address starport = admin();
	env e;
	calldataarg args;
	require e.msg.sender == starport;

	uint128 startNextIndex;
	_, startNextIndex = nextCashYieldAndIndex();

	f(e, args);

	uint128 endNextIndex;
	_, endNextIndex = nextCashYieldAndIndex();

	assert startNextIndex == endNextIndex;
}

/**
 * An attempt to characterize reasonable preconditions to amountToPrincipal() to help
 * figure out transferStandardPrecondition
 */
rule amountToPrincipalCharacterizeRevert(uint256 amount) {
	env e;
	require e.msg.value == 0;
	require amount * indexBaseUnit@norevert() < MAX_UINT128();
	amountToPrincipal@withrevert(e, amount);
	assert !lastReverted;
}