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

    uint public constant SECONDS_PER_YEAR = 31536000;

    address immutable public admin;
    uint public cashYieldStartAt;
    CashYieldAndIndex public cashYieldAndIndex;
    uint public nextCashYieldStartAt;
    CashYieldAndIndex public nextCashYieldAndIndex;
    mapping (address => mapping (address => uint)) public allowances;
    uint public totalCashPrincipal;
    mapping (address => uint128) public cashPrincipal;

	constructor(address starport, uint128 initialYield, uint128 initialYieldIndex, uint initialYieldStart) {
        admin = starport;
        // Note: we don't check that this is in the past, but calls will revert until it is.
        cashYieldStartAt = initialYieldStart;
        cashYieldAndIndex = CashYieldAndIndex({yield: initialYield, index: initialYieldIndex});
	}

    /**
     * Section: Ethereum Asset Interface
     */

    /**
     * @notice Mint Cash tokens for the given account
     * @dev Invoked by Starport contract
     * @dev principal is `u128` to be compliant with Compound Chain
     * @param account The owner of minted Cash tokens
     * @param principal The principal amount of minted Cash tokens
     */
    function mint(address account, uint128 principal) external override returns (uint) {
        require(msg.sender == admin, "Must be admin");
        uint amount = principal * getCashIndex() / 1e18;
        cashPrincipal[account] += principal;
        totalCashPrincipal = totalCashPrincipal + principal;
        emit Transfer(address(0), account, amount);
        return amount;
    }

    /**
     * @notice Burn Cash tokens for the given account
     * @dev Invoked by Starport contract
     * @dev principal is `u128` to be compliant with Compound Chain
     * @param account The owner of burned Cash tokens
     * @param amount The amount of burned Cash tokens
     */
    function burn(address account, uint amount) external override returns (uint128) {
        require(msg.sender == admin, "Must be admin");
        uint128 principal = amountToPrincipal(amount);
        cashPrincipal[account] -= principal;
        totalCashPrincipal = totalCashPrincipal - principal;
        emit Transfer(account, address(0), amount);
        return principal;
    }

    /**
     * @notice Update yield and index to be in sync with Compound chain
     * @dev It is expected to be called at least once per day
     * @dev Cash index denomination is 1e18
     * @param nextYield The new value of Cash APY measured in BPS
     * @param nextIndex The new value of Cash index
     * @param nextYieldStartAt The timestamp when new values for cash and index are activated
     */
    function setFutureYield(uint128 nextYield, uint128 nextIndex, uint nextYieldStartAt) external override {
        require(msg.sender == admin, "Must be admin");
        uint nextAt = nextCashYieldStartAt;

        // Updating cash yield and index to the 'old' next values
        if (nextAt != 0 && block.timestamp > nextAt) {
            cashYieldStartAt = nextAt;
            cashYieldAndIndex = nextCashYieldAndIndex;
        }
        nextCashYieldStartAt = nextYieldStartAt;
        nextCashYieldAndIndex = CashYieldAndIndex({yield: nextYield, index: nextIndex});
    }

    /**
     * @notice Get current cash index
     * @dev Since function is `view` and cannot modify storage,
            the check for next index and yield values was added
     */
    function getCashIndex() public view virtual override returns (uint) {
        uint nextAt = nextCashYieldStartAt;
        if (nextAt != 0 && block.timestamp > nextAt) {
            return calculateIndex(nextCashYieldAndIndex.yield, nextCashYieldAndIndex.index, nextAt);
        } else {
            return calculateIndex(cashYieldAndIndex.yield, cashYieldAndIndex.index, cashYieldStartAt);
        }
    }

    /**
     * Section: ERC20 Interface
     */

    function totalSupply() external view override returns (uint) {
        return totalCashPrincipal * getCashIndex() / 1e18;
    }

    function balanceOf(address account) view external override returns (uint) {
        return cashPrincipal[account] * getCashIndex() / 1e18;
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

    /**
     * Section: Function Helpers
     */

    // Helper function to conver amount to principal using current Cash index
    function amountToPrincipal(uint amount) public view returns (uint128) {
        uint256 principal = amount * 1e18 / getCashIndex();
        require(principal < type(uint128).max, "amountToPrincipal::overflow");
        return uint128(principal);
    }

    // Helper function to calculate current Cash index
    // Note: Formula for continuos compounding interest -> A = Pe^rt,
    //       current_index = base_index * e^(yield * time_ellapsed)
    //       yield is in BPS, so 300 = 3% = 0.03
    // TODO: check if it's really safe if time_elapsed > 1 day and yield is high
    function calculateIndex(uint yield, uint index, uint startAt) public view returns (uint) {
        return index * exponent(yield, block.timestamp - startAt) / 1e18;
    }

    // Helper function to calculate e^rt part from countinous compounding interest formula
    // Note: We use the third degree approximation of Taylor Series
    //       1 + x/1! + x^2/2! + x^3/3!
    // TODO: check if it's really safe if time_elapsed > 1 day and yield is high
    function exponent(uint yield, uint time) public view returns (uint) {
        uint epower = yield * time * 1e14 / SECONDS_PER_YEAR;
        uint first = epower;
        uint second = epower * epower / 2e18;
        uint third = epower * epower * epower / 6e36;
        return 1e18 + first + second + third;
    }
}
