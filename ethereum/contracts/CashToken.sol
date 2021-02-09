pragma solidity ^0.8.1;

import "./ICash.sol";

contract CashToken is ICash {
    struct NextCashYieldAndIndex {
        uint128 yield;
        uint128 index;
    }

    address immutable public admin;
    mapping (address => mapping (address => uint)) internal allowances;
    uint public totalCashPrincipal;
    mapping (address => uint256) internal cashPrincipal;
    uint cashYieldStartAt;
    uint128 cashYield;
    uint128 cashIndex;
    uint nextCashYieldStartAt;
    NextCashYieldAndIndex nextCashYieldAndIndex;

    uint constant exponent = 271828;

	constructor(address starport) {
		admin = starport;
	}

    function mint(address account, uint amountPrincipal) external {
        require(msg.sender == admin, "Sender is not an admin");
        uint amount = amountPrincipal * fetchCashIndex();
        cashPrincipal[account] = cashPrincipal[account] + amount;
        totalCashPrincipal = totalCashPrincipal + amount;
        emit Transfer(address(0), account, amount);
    }

    function burn(address account, uint amountPrincipal) external {
        require(msg.sender == admin, "Sender is not an admin");
        uint amount = amountPrincipal * fetchCashIndex();
        cashPrincipal[account] = cashPrincipal[account] - amount;
        totalCashPrincipal = totalCashPrincipal - amount;
        emit Transfer(account, address(0), amount);
    }

    function setFutureYield(uint128 nextYield, uint nextYieldStartAt, uint128 nextIndex) external {
        require(msg.sender == admin, "Sender is not an admin");
        nextCashYieldStartAt = nextYieldStartAt;
        nextCashYieldAndIndex = NextCashYieldAndIndex(nextYield, nextIndex);
    }

    function fetchCashIndex() public returns (uint) {
        uint nextAt = nextCashYieldStartAt;
        if (now > nextAt) {
            cashYieldStartAt = nextAt;
            cashYield = nextCashYieldAndIndex.yield;
            cashIndex = nextCashYieldAndIndex.index;
            nextCashYieldStartAt = 0;
        }
        // TODO work more on this formula
        return cashIndex * (271828 ** cashYield * (now - cashYieldStartAt)) / 100000;
    }

    function totalSupply() public returns (uint) {
        return totalCashPrincipal * fetchCashIndex();
    }

    function balanceOf(address account) public returns (uint) {
        return cashPrincipal[account] * fetchCashIndex();
    }

    function transfer(address recipient, uint amount) public returns (bool) {
        require(msg.sender != recipient, "Invalid recipient");
        uint principal = amount / fetchCashIndex();
        cashPrincipal[recipient] = cashPrincipal[recipient] + principal;
        cashPrincipal[msg.sender] = cashPrincipal[msg.sender] - principal;
        emit Transfer(msg.sender, recipient, principal);
        return true;
    }

    function allowance(address owner, address spender) public view returns (uint256) {
        return allowances[owner][spender];
    }

    function approve(address spender, uint amount) public view returns (bool) {
        allowances[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(address sender, address recipient, uint256 amount) public virtual override returns (bool) {
        require(sender != recipient, "Invalid recipient");
        address spender = msg.sender;
        uint principal = amount / fetchCashIndex();
        allowances[sender][spender] = allowances[sender][spender] - amount;
        cashPrincipal[recipient] = cashPrincipal[recipient] + principal;
        cashPrincipal[sender] = cashPrincipal[sender] - principal;
        emit Transfer(msg.sender, recipient, principal);
        return true;
    }

    /**
     * @dev Returns the name of the token.
     */
    function name() public view virtual returns (string memory) {
        return "SECRET, change";
    }

    /**
     * @dev Returns the symbol of the token, usually a shorter version of the
     * name.
     */
    function symbol() public view virtual returns (string memory) {
        return "SECRET";
    }

    function decimals() public view virtual returns (uint8) {
        return 6;
    }
}