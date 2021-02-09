// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

/**
 * @title Generic Erc-20 Interface
 */
interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
}

/**
 * @title Generic Cash Token Interface
 */
interface ICash is IERC20 {
    function burn(uint256 amount) external;
    function fetchHypotheticalIndex() external returns (uint);
}
