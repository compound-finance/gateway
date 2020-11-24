pragma solidity ^0.7.5;

import "./IERC20.sol";

// via https://github.com/dapphub/ds-math/blob/master/src/math.sol
function add(uint x, uint y) pure returns (uint z) {
    require((z = x + y) >= x, "ds-math-add-overflow");
}

function sub(uint x, uint y) pure returns (uint z) {
    require((z = x - y) <= x, "ds-math-sub-underflow");
}

function mul(uint x, uint y) pure returns (uint z) {
    require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
}

contract Starport {

	ICash immutable public cash;

	event LockCash(address holder, uint amount, uint yieldIndex);
	event Lock(address asset, address holder, uint amount);
	event LockETH(address holder, uint amount);

	constructor(ICash cash_) {
		cash = cash_;
	}

	function lock(uint amount, address asset) public {
		if (asset == address(cash)) {
			lockCashInternal(amount, msg.sender);
		} else {
			lockInternal(amount, asset, msg.sender);
		}
	}

	function lockETH() public payable {
		// TODO: Check Supply Cap
		emit LockETH(msg.sender, msg.value);
	}

	function lockCashInternal(uint amount, address sender) internal {
		// cash.burn(amount);
		uint yieldIndex = cash.fetchHypotheticalIndex();
		transferInCash(sender, amount);
		emit LockCash(sender, amount, yieldIndex);
	}

	function lockInternal(uint amount, address asset, address sender) internal {
		// TODO: Check Supply Cap
		uint amountTransferred = transferIn(sender, amount, IERC20(asset));
		emit Lock(asset, sender, amountTransferred);
	}

	// Make sure that the amount we ask for
	function transferIn(address from, uint amount, IERC20 asset) internal returns (uint) {
		uint balBefore = asset.balanceOf(address(this));
		require(asset.transferFrom(from, address(this), amount) == true, "TransferIn");
		uint balAfter = asset.balanceOf(address(this));
		return sub(balAfter, balBefore);
	}

	function transferInCash(address from, uint amount) internal {
		require(cash.transferFrom(from, address(this), amount) == true, "TransferInCash");
	}

	receive() external payable {
		lockETH();
	}
}
