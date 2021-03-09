// SPDX-License-Identifier: GPL-3.0

pragma solidity ^0.8.1;

/**
 * @title Generic Erc-20 Interface
 */
interface IERC20 {
    function totalSupply() external view returns (uint256);
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function approve(address spender, uint amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

/**
 * @title Generic Cash Token Interface
 */
interface ICash is IERC20 {
    function mint(address account, uint128 principal) external returns (uint);
    function burn(address account, uint amount) external returns (uint128);
    function setFutureYield(uint128 nextYield, uint128 nextIndex, uint nextYieldStartAt) external;
    function getCashIndex() external view returns (uint128);
}

/**
 * @title Non-Standard Erc-20 Interface for tokens which do not return from `transfer` or `transferFrom`
 */
interface INonStandardERC20 {
    function transfer(address recipient, uint256 amount) external;
    function transferFrom(address sender, address recipient, uint256 amount) external;
    function balanceOf(address account) external view returns (uint256);
}