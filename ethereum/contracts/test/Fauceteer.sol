// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../ICash.sol";

/**
  * @title Compound Fauceteer
  * @author Compound Finance
  * @notice Contract to distribute test-net tokens via `drip`
  */
contract Fauceteer {

    /**
      * @notice Drips some tokens to caller
      * @dev We send 0.01% of our tokens to the caller.
      * @dev Over time, the amount will tend toward and eventually reach zero.
      * @dev Note: if we have no balance in this token, function will revert.
      * @param token The token to drip.
      */
    function drip(INonStandardERC20 token) public {
        uint currBalance = token.balanceOf(address(this));
        require(currBalance > 0, "Fauceteer is empty");
        token.transfer(msg.sender, currBalance / 10000); // 0.01%

        bool success;
        assembly {
            switch returndatasize()
                case 0 {                       // This is a non-standard ERC-20
                    success := not(0)          // set success to true
                }
                case 32 {                      // This is a compliant ERC-20
                    returndatacopy(0, 0, 32)
                    success := mload(0)        // Set `success = returndata` of external call
                }
                default {                      // This is an excessively non-compliant ERC-20, revert.
                    revert(0, 0)
                }
        }

        require(success, "Transfer returned false.");
    }
}
