pragma solidity ^0.7.5;


// TODO: finish implementing ERC20
contract CashToken {

	address immutable public admin;
	mapping(address => uint256) balances;

	constructor(address admin_) {
		admin = admin_;
	}

	// TODO: implement
	// function getHypotheticalIndex() public view returns (uint)

	function balanceOf(address _owner) public view returns (uint256) {
        return balances[_owner];
    }


    // TODO: actually implement CASH principal balances etc
    function transferFrom(
        address _from,
        address _to,
        uint256 _value
    )
        public
        returns (bool)
    {
        // require(_to != address(0), "TransferFrom: Can't send to address zero");
        // require(_value <= balances[_from], "TransferFrom: Inadequate balance");
        // require(_value <= allowed[_from][msg.sender], "TransferFrom: Inadequate allowance");

        balances[_from] -= _value;
        balances[_to] += _value;
        // allowed[_from][msg.sender] = allowed[_from][msg.sender].sub(_value);
        // emit Transfer(_from, _to, _value);
        return true;
    }

}
