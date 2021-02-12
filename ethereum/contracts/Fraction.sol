pragma solidity 0.8.1;

/**
 * @title Fraction
 * @author dYdX
 *
 * This library contains implementations for fraction structs.
 */
library Fraction {
    struct Fraction128 {
        uint128 num;
        uint128 den;
    }
}