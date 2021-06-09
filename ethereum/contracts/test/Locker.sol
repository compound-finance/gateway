// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

import "../Starport.sol";
import "./TrxRequestBuilder.sol";

contract Locker is TrxRequestBuilder {
  Starport public immutable starport;

  constructor(address payable starport_) {
    starport = Starport(starport_);
  }

  receive () external payable {
    lockAndtransfer();
  }

  function lockAndtransfer() public payable {
    address recipient = msg.sender;
    string memory trxRequest = transferCashMaxRequest(recipient);
    bytes32 recipient32 = starport.toBytes32(recipient);
    starport.execTrxRequest(trxRequest);
    starport.lockEthTo{value: msg.value}("ETH", recipient32);
  }
}
