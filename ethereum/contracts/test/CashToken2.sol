// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../CashToken.sol";

contract CashToken2 is CashToken {
	bool public initialized_ = false;
	uint public counter = 0;

	constructor(address admin_) CashToken(admin_) {
	}

	function initialize_(uint counter_) public {
		require(initialized_ == false, "cannot reinitialize");
		counter = counter_;
		initialized_ = true;
	}

	/// Simple function to test notices
	function count_() public returns (uint) {
		return counter += 1;
	}
}
