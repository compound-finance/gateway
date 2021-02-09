// SPDX-License-Identifier: GPL-3.0

pragma solidity ^0.8.1;

interface IERC20 {
    function totalSupply() external returns (uint256);
    function balanceOf(address account) external returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function allowance(address owner, address spender) external view returns (uint256);
    function approve(address spender, uint amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);
}

interface ICash is IERC20 {
    function mint(address account, uint amountPrincipal) external;
    function burn(address account, uint amountPrincipal) external;
    function setFutureYield(uint128 nextYield, uint nextYieldStartAt, uint128 nextIndex) external;
    function fetchCashIndex() external returns (uint);
}
