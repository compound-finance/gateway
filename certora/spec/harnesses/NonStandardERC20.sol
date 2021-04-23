// SPDX-License-Identifier: agpl-3.0
pragma solidity ^0.8.1;

import '../../contracts/ICash.sol';
// with mint
contract NonStandardERC20 is INonStandardERC20 {
    uint256 t;
    mapping (address => uint256) b;
    mapping (address => mapping (address => uint256)) a;

    string public name;
    string public symbol;
    uint public decimals;

    function myAddress() public returns (address) {
        return address(this);
    }

    function add(uint a, uint b) internal pure returns (uint256) {
        uint c = a +b;
        require (c >= a);
        return c;
    }
    function sub(uint a, uint b) internal pure returns (uint256) {
        require (a>=b);
        return a-b;
    }

    function totalSupply() external view returns (uint256) {
        return t;
    }
    function balanceOf(address account) override external view returns (uint256) {
        return b[account];
    }
    function transfer(address recipient, uint256 amount) override external {
        b[msg.sender] = sub(b[msg.sender], amount);
        b[recipient] = add(b[recipient], amount);
    }
    function allowance(address owner, address spender) external view returns (uint256) {
        return a[owner][spender];
    }
    function approve(address spender, uint256 amount) external returns (bool) {
        a[msg.sender][spender] = amount;
        return true;
    }

    function transferFrom(
        address sender,
        address recipient,
        uint256 amount
    ) override external {
        b[sender] = sub(b[sender], amount);
        b[recipient] = add(b[recipient], amount);
        a[sender][msg.sender] = sub(a[sender][msg.sender], amount);
    }
}