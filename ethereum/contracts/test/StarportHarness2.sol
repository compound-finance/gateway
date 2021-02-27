// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "./StarportHarness.sol";

contract StarportHarness2 is StarportHarness {
	bool public initialized_ = false;

	constructor(ICash cash_, address admin_) StarportHarness(cash_, admin_) {
	}

	function initialize_(uint counter_) public {
		require(initialized_ == false, "cannot reinitialize");
		counter += counter_;
		initialized_ = true;
	}

	function mul_(uint amt) public returns (uint) {
		return counter *= amt;
	}
}