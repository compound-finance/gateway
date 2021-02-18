// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "./ICash.sol";

/**
 * @title Compound Cash Token
 * @author Compound Finance
 * @notice The Compound Cash Token for Ethereum
 */
contract CashToken is ICash {
    struct CashYieldAndIndex {
        uint128 yield;
        uint128 index;
    }

    address immutable public admin;
    uint public cashYieldStartAt;
    CashYieldAndIndex public cashYieldAndIndex;
    uint public nextCashYieldStartAt;
    CashYieldAndIndex public nextCashYieldAndIndex;

    uint public constant SECONDS_PER_YEAR = 31536000;

    mapping (address => mapping (address => uint)) public allowances;
    uint public totalCashPrincipal;
    mapping (address => uint128) public cashPrincipal;

	constructor(address starport, uint128 initialYield, uint128 initialYieldIndex, uint initialYieldStart) {
        admin = starport;
        // Note: we don't check that this is in the past, but calls will revert until it is.
        cashYieldStartAt = initialYieldStart;
        cashYieldAndIndex = CashYieldAndIndex({yield: initialYield, index: initialYieldIndex});
	}

    function mint(address account, uint128 principal) external override returns (uint) {
        require(msg.sender == admin, "Must be admin");
        uint amount = principal * getCashIndex();
        cashPrincipal[account] += principal;
        totalCashPrincipal = totalCashPrincipal + principal;
        emit Transfer(address(0), account, amount);
        return amount;
    }

    function burn(address account, uint amount) external override returns (uint128) {
        require(msg.sender == admin, "Must be admin");
        uint128 principal = amountToPrincipal(amount);
        cashPrincipal[account] -= principal;
        totalCashPrincipal = totalCashPrincipal - principal;
        emit Transfer(account, address(0), amount);
        return principal;
    }

    function setFutureYield(uint128 nextYield, uint128 nextIndex, uint nextYieldStartAt) external override {
        require(msg.sender == admin, "Must be admin");
        uint nextAt = nextCashYieldStartAt;
        if (nextAt != 0 && block.timestamp > nextAt) {
            cashYieldStartAt = nextAt;
            cashYieldAndIndex = nextCashYieldAndIndex;
        }
        nextCashYieldStartAt = nextYieldStartAt;
        nextCashYieldAndIndex = CashYieldAndIndex({yield: nextYield, index: nextIndex});
    }

    function getCashIndex() public view virtual override returns (uint) {
        uint nextAt = nextCashYieldStartAt;
        if (nextAt != 0 && block.timestamp > nextAt) {
            return calculateIndex(nextCashYieldAndIndex.yield, nextCashYieldAndIndex.index, nextAt);
        } else {
            return calculateIndex(cashYieldAndIndex.yield, cashYieldAndIndex.index, cashYieldStartAt);
        }
    }

    function totalSupply() external view override returns (uint) {
        return totalCashPrincipal * getCashIndex();
    }

    function balanceOf(address account) external view override returns (uint) {
        return cashPrincipal[account] * getCashIndex();
    }

    function transfer(address recipient, uint amount) external override returns (bool) {
        require(msg.sender != recipient, "Invalid recipient");
        uint128 principal = amountToPrincipal(amount);
        cashPrincipal[recipient] += principal;
        cashPrincipal[msg.sender] -= principal;
        emit Transfer(msg.sender, recipient, amount);
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
        uint128 principal = amountToPrincipal(amount);
        allowances[sender][spender] -= amount;
        cashPrincipal[recipient] += principal;
        cashPrincipal[sender] -= principal;
        emit Transfer(sender, recipient, amount);
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

    function amountToPrincipal(uint amount) public view returns (uint128) {
        uint256 principal = amount / getCashIndex();
        require(principal < type(uint128).max, "amountToPrincipal::overflow");
        return uint128(principal);
    }

    function calculateIndex(uint yield, uint index, uint startAt) internal view returns (uint) {
        return index * exponent(yield, block.timestamp - startAt) / 1e18;
    }

    // First precision of tayler series
    function exponent(uint yield, uint time) internal view returns (uint) {
        uint epower = yield * time * 1e13 / SECONDS_PER_YEAR;
        return 1e18 + epower;
    }
}
