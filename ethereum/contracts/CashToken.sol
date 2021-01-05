pragma solidity ^0.7.5;


// TODO: finish implementing ERC20
contract CashToken {

    /// @notice EIP-20 token name for this token
    string public constant name = "Widget Token";

    /// @notice EIP-20 token symbol for this token
    string public constant symbol = "WDGT";

    /// @notice EIP-20 token decimals for this token
    uint8 public constant decimals = 18;

    /// @notice Allowance amounts on behalf of others
    mapping (address => mapping (address => uint96)) internal allowances;

    /// @notice Official record of token balances for each account
    mapping (address => uint96) internal balances;

    /// @notice The standard EIP-20 transfer event
    event Transfer(address indexed from, address indexed to, uint256 amount);

    /// @notice The standard EIP-20 approval event
    event Approval(address indexed owner, address indexed spender, uint256 amount);

	address immutable public admin;

	constructor(address admin_) {
		admin = admin_;
	}

    /**
     * @notice Get the number of tokens `spender` is approved to spend on behalf of `account`
     * @param account The address of the account holding the funds
     * @param spender The address of the account spending the funds
     * @return The number of tokens approved
     */
    function allowance(address account, address spender) external view returns (uint256) {
        return allowances[account][spender];
    }

    /**
     * @notice Approve `spender` to transfer up to `amount` from `src`
     * @dev This will overwrite the approval amount for `spender`
     *  and is subject to issues noted [here](https://eips.ethereum.org/EIPS/eip-20#approve)
     * @param spender The address of the account which may transfer tokens
     * @param amount The number of tokens that are approved (2^256-1 means infinite)
     * @return Whether or not the approval succeeded
     */
    function approve(address spender, uint256 amount) external returns (bool) {
        allowances[msg.sender][spender] = amount;

        emit Approval(msg.sender, spender, amount);
        return true;
    }

	// TODO: implement
	// function getHypotheticalIndex() public view returns (uint)

    /**
     * @notice Get the number of tokens held by the `account`
     * @param account The address of the account to get the balance of
     * @return The number of tokens held
     */
    function balanceOf(address account) external view returns (uint256) {
        return balances[account];
    }

    /**
     * @notice Transfer `amount` tokens from `msg.sender` to `dst`
     * @param dst The address of the destination account
     * @param amount The number of tokens to transfer
     * @return Whether or not the transfer succeeded
     */
    function transfer(address dst, uint amount) external returns (bool) {
        _transferTokens(msg.sender, dst, amount);
        return true;
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