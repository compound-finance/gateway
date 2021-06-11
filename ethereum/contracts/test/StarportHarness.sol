// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../Starport.sol";

contract StarportHarness is Starport {

	uint public counter;

	constructor(ICash cash_, address admin_) Starport(cash_, admin_, "ETH", "ETH:") {
	}

	/// Harness to call `getQuorum`
	function getQuorum_(uint authorityCount) external pure returns (uint) {
		return getQuorum(authorityCount);
	}

	/// Harness to call `checkNoticeSignerAuthorized`
	function checkNoticeSignerAuthorized_(
        bytes32 noticeHash,
        address[] memory authorities_,
        bytes[] calldata signatures
    ) external pure {
		return checkNoticeSignerAuthorized(noticeHash, authorities_, signatures);
	}

	/// Harness to call `mint` on Cash Token
	function mint_(address holder, uint128 principal) external {
		cash.mint(holder, principal);
	}

	/// Harness to accept ether
	function receive_() external payable {
	}

	/// Harness to call `recover`
	function recover_(bytes32 digest, bytes memory signature) public pure returns (address) {
		return recover(digest, signature);
	}

	/// Harness to call `unlock` with this as `msg.sender`
	function unlock_(address asset, uint amount, address payable account) external {
		Starport(this).unlock(asset, amount, account);
	}

	/// Harness to call `unlockCash` with this as `msg.sender`
	function unlockCash_(address account, uint128 principal) external {
		Starport(this).unlockCash(account, principal);
	}

	/// Simple function to test notices
	function count_() public returns (uint) {
		return counter += 1;
	}

	/// Simple function to test notices
	function revert_() pure public {
		require(false, "harness reversion");
	}

	/// Simple function to test notices
	function math_(uint a, uint8 b) pure public returns (uint) {
		return a + b * 10;
	}
}
