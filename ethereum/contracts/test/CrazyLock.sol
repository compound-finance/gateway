// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../Starport.sol";

contract CrazyLock {
  Starport public immutable starport;

  event CrazyLockRun(bytes32 recipient32, uint ts, bool doLock);

  constructor(address payable starport_) {
    starport = Starport(starport_);
  }

  function crazyLock(address recipient) public payable {
    uint ts = block.timestamp;
    bool doLock = ts % 2 == 0;
    bytes32 recipient32 = starport.toBytes32(recipient);

    emit CrazyLockRun(recipient32, ts, doLock);

    if (doLock) {
      starport.lockEthTo{value: msg.value}("ETH", recipient32);
    }
  }
}
