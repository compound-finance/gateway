// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

/**
 * @title ERC20 interface
 * @dev see https://github.com/ethereum/EIPs/issues/20
 */
abstract contract ERC20  {
    function totalSupply() public view virtual returns (uint256);
    function balanceOf(address who) public view virtual returns (uint256);
    function transfer(address to, uint256 value) public virtual returns (bool);
    function allowance(address owner, address spender) public view virtual returns (uint256);
    function transferFrom(address from, address to, uint256 value) public virtual returns (bool);
    function approve(address spender, uint256 value) public virtual returns (bool);

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

/**
 * @title Base ERC20 token
 *
 * @dev Implementation of the basic standard token.
 * https://github.com/ethereum/EIPs/issues/20
 * Based on code by FirstBlood: https://github.com/Firstbloodio/token/blob/master/smart_contract/FirstBloodToken.sol
 */
contract BaseToken {
    uint256 totalSupply_;

    string public name;
    string public symbol;
    uint8 public decimals;

    mapping (address => mapping (address => uint256)) internal allowed;
    mapping(address => uint256) balances;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    constructor(uint256 _initialAmount, string memory _tokenName, uint8 _decimalUnits, string memory _tokenSymbol) {
        totalSupply_ = _initialAmount;
        balances[msg.sender] = _initialAmount;
        name = _tokenName;
        symbol = _tokenSymbol;
        decimals = _decimalUnits;
    }


    function totalSupply() public view returns (uint256) {
        return totalSupply_;
    }


    function transfer_(address _to, uint256 _value) internal returns (bool) {
        require(_to != address(0));
        require(_value <= balances[msg.sender], "Transfer: insufficient balance");

        balances[msg.sender] = balances[msg.sender] - _value;
        balances[_to] = balances[_to] + _value;
        emit Transfer(msg.sender, _to, _value);
        return true;
    }

    function balanceOf(address _owner) public view returns (uint256) {
        return balances[_owner];
    }

    function transferFrom_(address _from, address _to, uint256 _value) internal returns (bool) {
        require(_to != address(0), "TransferFrom: Can't send to address zero");
        require(_value <= balances[_from], "TransferFrom: Inadequate balance");
        require(_value <= allowed[_from][msg.sender], "TransferFrom: Inadequate allowance");

        balances[_from] = balances[_from] - _value;
        balances[_to] = balances[_to] + _value;
        allowed[_from][msg.sender] = allowed[_from][msg.sender] - _value;
        emit Transfer(_from, _to, _value);
        return true;
    }


    function approve(address _spender, uint256 _value) public returns (bool) {
        allowed[msg.sender][_spender] = _value;
        emit Approval(msg.sender, _spender, _value);
        return true;
    }


    function allowance(address _owner, address _spender) public view returns (uint256) {
        return allowed[_owner][_spender];
    }

}

/**
 * @title The Compound Standard Test Token
 * @author Compound
 */
contract StandardToken is BaseToken {
    constructor(uint256 _initialAmount, string memory _tokenName, uint8 _decimalUnits, string memory _tokenSymbol)
        BaseToken(_initialAmount, _tokenName, _decimalUnits, _tokenSymbol)
    {
    }

    function transfer(address _to, uint256 _value) public returns (bool) {
        return transfer_(_to, _value);
    }

    function transferFrom(address _from, address _to, uint256 _value) public returns (bool) {
        return transferFrom_(_from, _to, _value);
    }
}

/**
 * @title The Compound Faucet Test Token
 * @author Compound
 * @notice A simple test token that lets anyone get more of it.
 */
contract FaucetToken is StandardToken {
    constructor(uint256 _initialAmount, string memory _tokenName, uint8 _decimalUnits, string memory _tokenSymbol)
        StandardToken(_initialAmount, _tokenName, _decimalUnits, _tokenSymbol)
    {
    }

    function allocateTo(address _owner, uint256 value) public {
        balances[_owner] += value;
        totalSupply_ += value;
        emit Transfer(address(this), _owner, value);
    }
}

/**
 * @title Fee ERC20 token
 */
contract FeeToken is BaseToken {
    uint256 constant FEE_BPS = 500; // 5%

    constructor(uint256 _initialAmount, string memory _tokenName, uint8 _decimalUnits, string memory _tokenSymbol) BaseToken(_initialAmount, _tokenName, _decimalUnits, _tokenSymbol) {
    }

    function transfer(address _to, uint256 _value) public returns (bool) {
        bool result = super.transfer_(_to, _value);

        uint256 fee = FEE_BPS * _value / 1000;
        balances[_to] -= fee;
        balances[address(0)] += fee;

        return result;
    }

    function transferFrom(address _from, address _to, uint256 _value) public returns (bool) {
        bool result = super.transferFrom_(_from, _to, _value);

        uint256 fee = FEE_BPS * _value / 1000;
        balances[_to] -= fee;
        balances[address(0)] += fee;

        return result;
    }
}

/**
 * @title Non-Standard ERC20 token
 */
contract NonStandardToken is BaseToken {
    constructor(uint256 _initialAmount, string memory _tokenName, uint8 _decimalUnits, string memory _tokenSymbol) BaseToken(_initialAmount, _tokenName, _decimalUnits, _tokenSymbol) {
    }

    function transfer(address _to, uint256 _value) public {
        super.transfer_(_to, _value);
    }

    function transferFrom(address _from, address _to, uint256 _value) public {
        super.transferFrom_(_from, _to, _value);
    }
}
