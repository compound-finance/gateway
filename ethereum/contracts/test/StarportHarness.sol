// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../Starport.sol";

contract StarportHarness is Starport {

	uint public counter;

	constructor(ICash cash_, address[] memory authorities_) Starport(cash_, authorities_) {
	}

	/// Harness to call `getQuorum`
	function getQuorum_(uint authorityCount) external pure returns (uint) {
		return getQuorum(authorityCount);
	}

	/// Harness to call `checkNoticeAuthorized`
	function checkNoticeAuthorized_(
        bytes calldata notice,
        address[] memory authorities_,
        bytes[] calldata signatures
    ) external view {
		return checkNoticeAuthorized(notice, authorities_, signatures);
	}

	/// Harness to call `recover`
	function recover_(bytes32 digest, bytes memory signature) public pure returns (address) {
		return recover(digest, signature);
	}

	/// Harness to call `unlock`
	function unlock_(address asset, uint amount, address account) external {
		Starport(this).unlock(asset, amount, account);
	}

	/// Harness to call `changeAuthorities`
	function changeAuthorities_(address[] calldata newAuthorities) external {
		Starport(this).changeAuthorities(newAuthorities);
	}

	/// Simple function to test notices
	function count_() public returns (uint) {
		return counter += 1;
	}
}
