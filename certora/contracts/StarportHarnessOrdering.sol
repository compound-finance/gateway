pragma solidity ^0.8.1;
pragma abicoder v2;

import './StarportHarness.sol';


contract StarportHarnessOrdering is StarportHarness {
    constructor(ICash cash_, address admin_) StarportHarness(cash_, admin_) {}

    function invokeNoticeInternal(bytes calldata notice, bytes32 noticeHash) override internal returns (bytes memory) {
        if (isNoticeInvoked[noticeHash]) {
            emit NoticeReplay(noticeHash);
            return "";
        }

        isNoticeInvoked[noticeHash] = true;

        require(notice.length >= 100, "Must have full header"); // 4 + 3 * 32
        require(notice[0] == MAGIC_HEADER[0], "Invalid header[0]");
        require(notice[1] == MAGIC_HEADER[1], "Invalid header[1]");
        require(notice[2] == MAGIC_HEADER[2], "Invalid header[2]");
        require(notice[3] == MAGIC_HEADER[3], "Invalid header[3]");

        (uint noticeEraId, uint noticeEraIndex, bytes32 noticeParent) =
            abi.decode(notice[4:100], (uint, uint, bytes32));

        noticeParent; // unused

        bool startNextEra = noticeEraId == eraId + 1 && noticeEraIndex == 0;

        require(
            noticeEraId <= eraId || startNextEra,
            "Notice must use existing era or start next era"
        );

        if (startNextEra) {
            eraId++;
        }

        // bytes memory calldata_ = bytes(notice[100:]);
        // (bool success, bytes memory callResult) = address(this).call(calldata_);
        // if (!success) {
        //     require(false, _getRevertMsg(callResult));
        // }

        // emit NoticeInvoked(uint32(noticeEraId), uint32(noticeEraIndex), noticeHash, callResult);

        // return callResult;

        return "success";
    }
}