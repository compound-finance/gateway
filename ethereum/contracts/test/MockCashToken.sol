// SPDX-License-Identifier: GPL-3.0

pragma solidity ^0.8.1;

import "../CashToken.sol";

contract MockCashToken is CashToken {

	constructor(address admin_, uint initialSupply_, address holder_) CashToken(admin_) {
		cashPrincipal[holder_] = initialSupply_;
		totalCashPrincipal = initialSupply_;
	}

	function fetchCashIndex() public pure override returns (uint) {
		return 1e18;
	}
}
