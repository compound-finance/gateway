pragma solidity ^0.7.5;



import "../CashToken.sol";

contract MockCashToken is CashToken {

	constructor(address admin_, uint initialSupply_, address holder_) CashToken(admin_) {
		balances[holder_] = initialSupply_;
	}

	function fetchHypotheticalIndex() public returns (uint) {
		return 1e18;
	}
}
