// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;

contract TrxRequestBuilder {
  function nibbleToHex(uint8 v) pure public returns (bytes1) {
    require(v < 0x10, "Nibble should be below 16");

    if (v < 10) {
      return bytes1(0x30 + v); // '0' + v
    } else {
      return bytes1(0x41 + (v - 10)); // 'A' + v
    }
  }

  function toHexByte(uint8 v) pure public returns (bytes1, bytes1) {
    bytes1 high = nibbleToHex((v >> 4) & 0xf);
    bytes1 low = nibbleToHex(v & 0x0f);
    return (high, low);
  }

  function toHex(address addr) pure public returns (string memory) {
    bytes20 address20 = bytes20(addr);
    bytes memory res = new bytes(40);
    for (uint i = 0; i < 20; i++) {
      (bytes1 high, bytes1 low) = toHexByte(uint8(address20[i]));
      res[i * 2] = high;
      res[i * 2 + 1] = low;
    }
    return string(res);
  }

  function strcpy(string memory src, string memory dest, uint offset) pure internal {
    bytes memory srcb = bytes(src);
    bytes memory destb = bytes(dest);

    require(offset + srcb.length <= destb.length, "strcpy beyond end of target");

    for (uint i = 0; i < srcb.length; i++) {
      destb[offset+i] = srcb[i];
    }
  }

  function transferCashMaxRequest(address recipient) pure public returns (string memory) {
    string memory res = "(Transfer MAX Cash Eth:0x0000000000000000000000000000000000000000)";
    strcpy(toHex(recipient), res, 25);
    return string(res);
  }
}
