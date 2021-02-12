// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "./ICash.sol";
import "./Exponent.sol";

/**
 * @title Compound Cash Token
 * @author Compound Finance
 * @notice The Compound Cash Token for Ethereum
 * @dev XXX Finish implementing ERC-20 features
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

    mapping (address => mapping (address => uint)) internal allowances;
    uint internal totalCashPrincipal;
    mapping (address => uint256) internal cashPrincipal;

	constructor(address starport) {
		admin = starport;
        cashYieldStartAt = block.timestamp;
        cashYieldAndIndex = CashYieldAndIndex({yield: 0, index: 1e6});
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
        cashPrincipal[account] = cashPrincipal[account] - amountPrincipal;
        totalCashPrincipal = totalCashPrincipal - amountPrincipal;
        emit Transfer(account, address(0), amount);
    }

    function setFutureYield(uint128 nextYield, uint128 nextIndex, uint nextYieldStartAt) external override {
        require(msg.sender == admin, "Sender is not an admin");
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
        }
        return calculateIndex(cashYieldAndIndex.yield, cashYieldAndIndex.index, cashYieldStartAt);
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
        uint principal = amount / getCashIndex();
        allowances[sender][spender] = allowances[sender][spender] - amount;
        cashPrincipal[recipient] = cashPrincipal[recipient] + principal;
        cashPrincipal[sender] = cashPrincipal[sender] - principal;
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

    function calculateIndex(uint yield, uint index, uint startAt) internal view returns (uint) {
        uint128 epower = uint128(yield * (block.timestamp - startAt) / 1e6);

        Fraction.Fraction128 memory percent = Exponent.exp(
            Fraction.Fraction128({
                num: epower,
                den: 365 * 24 * 60 * 60
            }),
            1,
            1
        );

        return index * percent.num / percent.den;
    }
}