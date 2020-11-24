pragma solidity ^0.7.5;



interface IERC20 {
    function balanceOf(address account) external view returns (uint256);
    function transfer(address recipient, uint256 amount) external returns (bool);
    function transferFrom(address sender, address recipient, uint256 amount) external returns (bool);
}

interface ICash is IERC20 {
    function burn(uint256 amount) external;
    function fetchHypotheticalIndex() external returns (uint);
}
