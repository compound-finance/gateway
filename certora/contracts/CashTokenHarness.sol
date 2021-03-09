pragma solidity ^0.8.1;
pragma abicoder v2;

import './CashToken.sol';


contract CashTokenHarness is CashToken {
    // yield -> index -> start -> timestamp -> uint128
    mapping(uint128 => mapping(uint128 => mapping(uint256 => mapping(uint256 => uint128)))) uninterpIndexCalc;
    // amount -> calculatedIndex -> uint128
    mapping(uint256 => mapping(uint128 => uint128)) uninterpPrincipalCalc;

    constructor(address admin_) CashToken(admin_) {}

    function calculateIndex(uint128 yield, uint128 index, uint start) override public view returns (uint128) {
        // uint128 newIndex = super.calculateIndex(yield, index, start);
        // require(newIndex == uninterpIndexCalc[yield][index][start][block.timestamp]);
        // return newIndex;
        return uninterpIndexCalc[yield][index][start][block.timestamp];
    }

    function amountToPrincipal(uint amount) override public returns (uint128) {
        uint128 principal = super.amountToPrincipal(amount);
        uninterpPrincipalCalc[amount][getCashIndex()] = principal;
        return principal;
    }
}