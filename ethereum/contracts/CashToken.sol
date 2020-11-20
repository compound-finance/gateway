pragma solidity ^0.6.10;

contract CashToken {

	function foo(uint a) public view returns (uint) {
		require(a == 4, "bar");
		return a;
	}

}
