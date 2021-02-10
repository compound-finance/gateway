// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "./ICash.sol";

/**
 * @title Compound Cash Token
 * @author Compound Finance
 * @notice The Compound Cash Token for Ethereum
 * @dev XXX Finish implementing ERC-20 features
 */
contract CashToken is ICash {
    struct NextCashYieldAndIndex {
        uint128 yield;
        uint128 index;
    }

    address immutable public admin;
    mapping (address => mapping (address => uint)) internal allowances;
    uint internal totalCashPrincipal;
    mapping (address => uint256) internal cashPrincipal;
    uint public cashYieldStartAt;
    uint128 public cashYield;
    uint128 public cashIndex;
    uint public nextCashYieldStartAt;
    NextCashYieldAndIndex public nextCashYieldAndIndex;

	constructor(address starport) {
		admin = starport;
        cashYieldStartAt = block.timestamp;
        cashYield = 43628;
        cashIndex = 1e6;
	}

    function mint(address account, uint amountPrincipal) external override {
        require(msg.sender == admin, "Sender is not an admin");
        uint amount = amountPrincipal * getCashIndex();
        cashPrincipal[account] = cashPrincipal[account] + amountPrincipal;
        totalCashPrincipal = totalCashPrincipal + amountPrincipal;
        emit Transfer(address(0), account, amount);
    }

    function burn(address account, uint amountPrincipal) external override {
        require(msg.sender == admin, "Sender is not an admin");
        uint amount = amountPrincipal * getCashIndex();
        cashPrincipal[account] = cashPrincipal[account] - amount;
        totalCashPrincipal = totalCashPrincipal - amount;
        emit Transfer(account, address(0), amount);
    }

    function setFutureYield(uint128 nextYield, uint nextYieldStartAt, uint128 nextIndex) external override {
        require(msg.sender == admin, "Sender is not an admin");
        uint nextAt = nextCashYieldStartAt;

        //@dev Cash yield is not set yet, first call of this method
        if (nextAt == 0) {
            cashYieldStartAt = nextYieldStartAt;
            cashYield = nextYield;
            cashIndex = nextIndex;
        } else if (block.timestamp > nextAt) {
            cashYieldStartAt = nextAt;
            cashYield = nextCashYieldAndIndex.yield;
            cashIndex = nextCashYieldAndIndex.index;
        }

        nextCashYieldStartAt = nextYieldStartAt;
        nextCashYieldAndIndex = NextCashYieldAndIndex(nextYield, nextIndex);
    }

    function getCashIndex() public view virtual override returns (uint) {
        uint nextAt = nextCashYieldStartAt;
        if (nextAt == 0) {
            return 1e6;
        }
        if (block.timestamp > nextAt) {
            return calculateIndex(nextCashYieldAndIndex.index, nextCashYieldAndIndex.yield, nextAt);
        }
        return calculateIndex(cashIndex, cashYield, cashYieldStartAt);
    }

    function totalSupply() external view override returns (uint) {
        return totalCashPrincipal * getCashIndex();
    }

    function balanceOf(address account) external view override returns (uint) {
        return cashPrincipal[account] * getCashIndex();
    }

    function transfer(address recipient, uint amount) external override returns (bool) {
        require(msg.sender != recipient, "Invalid recipient");
        uint principal = amount / getCashIndex();
        cashPrincipal[recipient] = cashPrincipal[recipient] + principal;
        cashPrincipal[msg.sender] = cashPrincipal[msg.sender] - principal;
        emit Transfer(msg.sender, recipient, principal);
        return true;
    }

    function allowance(address owner, address spender) external view override returns (uint256) {
        return allowances[owner][spender];
    }

    function approve(address spender, uint amount) external override returns (bool) {
        allowances[msg.sender][spender] = amount;
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function transferFrom(address sender, address recipient, uint256 amount) external override returns (bool) {
        require(sender != recipient, "Invalid recipient");
        address spender = msg.sender;
        uint principal = amount / getCashIndex();
        allowances[sender][spender] = allowances[sender][spender] - amount;
        cashPrincipal[recipient] = cashPrincipal[recipient] + principal;
        cashPrincipal[sender] = cashPrincipal[sender] - principal;
        emit Transfer(msg.sender, recipient, principal);
        return true;
    }

    /**
     * @dev Returns the name of the token.
     */
    function name() external pure returns (string memory) {
        return "SECRET, change";
    }

    /**
     * @dev Returns the symbol of the token, usually a shorter version of the
     * name.
     */
    function symbol() external pure returns (string memory) {
        return "SECRET";
    }

    function decimals() external pure returns (uint8) {
        return 6;
    }

    function calculateIndex(uint index, uint yield, uint startAt) internal view returns (uint) {
         // TODO work more on this formula
        return index * (271828 ** yield * (block.timestamp - startAt)) / 100000;
    }
}