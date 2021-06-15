// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../Starport.sol";

contract CrazyLock {
  Starport public immutable starport;

  event CrazyLockRun(bytes32 recipient32, bool doLock);

  bool halted;

  constructor(address payable starport_) {
    starport = Starport(starport_);
    halted = false;
  }

  function halt() public {
    halted = true;
  }

  function crazyLock(address recipient) public payable {
    bytes32 recipient32 = starport.toBytes32(recipient);

    emit CrazyLockRun(recipient32, halted);

    if (!halted) {
      starport.lockEthTo{value: msg.value}("ETH", recipient32);
    }
  }
}
