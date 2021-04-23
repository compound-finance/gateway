pragma solidity ^0.8.1;
pragma abicoder v2;

import './Starport.sol';


contract StarportHarness is Starport {
    constructor(ICash cash_, address admin_) Starport(cash_, admin_) {}

    address[] havocdAuthorities;
    bytes[] havocdSignatures;
    bytes[] havocdNotices;
    bytes havocdNotice;
    bool public isAssert;

    // for harness version of recover
    mapping(bytes32 => mapping(bytes => address)) recoverer;

    function noticeAlreadyInvoked() public returns (bool) {
        // force model of hash not to just havoc
        bytes32 noticeHash = this.hashNoticeExt(havocdNotice);
        return isNoticeInvoked[noticeHash];
    }

    function nowAssert() public {
        isAssert = true;
    }

    function invokeNotice(bytes calldata theNotice, bytes[] calldata signatures) public returns (bool) {
        if (isAssert) {
            bytes32 noticeHash = this.hashNoticeExt(theNotice);
            return isNoticeInvoked[noticeHash];
        } else {
            this.invoke(theNotice, signatures);
            return true;
        }
    }

    function invokeNoticeChain(bytes calldata theNotice, bytes[] calldata notices) public returns (bool) {
        if (isAssert) {
            bytes32 noticeHash = hashNotice(theNotice);
            return isNoticeInvoked[noticeHash];
        } else {
            this.invokeChain(theNotice, notices);
            return true;
        }
    }

    function partialOrderingCall(bytes calldata notice) public returns (uint256,uint256,bytes32) {
        if (isAssert) {
            bytes32 noticeHash = hashNotice(notice);
            uint256 returnSize = invokeNoticeInternal(notice, noticeHash).length;
            return (returnSize,0,0);
        } else {
            return myNoticeInfo(notice);
        }
    }

    function myNoticeInfo(bytes calldata notice) public returns (uint256,uint256,bytes32) {
        // copy-pasat from invokeNoticeInternal
        (uint noticeEraId, uint noticeEraIndex, bytes32 noticeParent) =
        abi.decode(notice[4:100], (uint, uint, bytes32));
        bytes32 noticeHash = hashNotice(notice);
        return (noticeEraId, noticeEraIndex, noticeHash);
    }

    function noticeReturnsEmptyString() public returns (bool) {
        return this.invokeNoticeInternalExt(havocdNotice, this.hashNoticeExt(havocdNotice)).length == 0;
    }

    function invokeNoticeInternalExt(bytes calldata notice, bytes32 noticeHash) public returns (bytes memory) {
        return invokeNoticeInternal(notice, noticeHash);
    }

    function balanceInERC20Asset(address asset) public view returns (uint256) {
        return IERC20(asset).balanceOf(address(this));
    }

    function signaturesAuthorizedPrecondition(uint256 nSignatures, uint256 nAuthorities, bytes32 noticeHash) public {
        require(havocdSignatures.length == nSignatures);
        require(havocdAuthorities.length == nAuthorities);
        for (uint i = 0; i < nSignatures; i++) {
            for (uint j = 0; j < nSignatures; j++) {
                if (i != j) {
                    require(recover(noticeHash, havocdSignatures[i]) != recover(noticeHash, havocdSignatures[j]));
                }
            }
            require(contains(havocdAuthorities, recover(noticeHash, havocdSignatures[i])));
        }
    }

    function checkSignatures(uint256 nSignatures, uint256 nAuthorities, bytes32 noticeHash) public {
        address[] memory authorities = new address[](nAuthorities);
        bytes[] memory signatures = new bytes[](nSignatures);
        for (uint i = 0; i < nSignatures; i++) {
            signatures[i] = havocdSignatures[i];
        }
        for (uint i = 0; i < nAuthorities; i++) {
            authorities[i] = havocdAuthorities[i];
        }
        this.checkSignaturesAuthorized(noticeHash, authorities, signatures);
    }

    function checkSignaturesAuthorized(bytes32 noticeHash, address[] memory authorities, bytes[] calldata signatures) public {
        checkNoticeSignerAuthorized(noticeHash, authorities, signatures);
    }

    function hashChainTargetNotParentPrecond(bytes32 targetHash) public {
        bytes memory firstNotice = havocdNotices[0];
        require(this.getParentHashExt(firstNotice) != targetHash);
    }

    function hashChainBrokenLinkPrecond(uint256 brokenLink) public {
        require(brokenLink + 1 < havocdNotices.length);
        bytes memory firstNotice = havocdNotices[brokenLink + 1];
        bytes memory shouldBeParentNotice = havocdNotices[brokenLink];
        bytes32 shouldBeParentHash = this.hashNoticeExt(shouldBeParentNotice);
        bytes32 parentHashOfFirstNotice = this.getParentHashExt(firstNotice);
        require(shouldBeParentHash != parentHashOfFirstNotice);
    }

    function getParentHashExt(bytes calldata theNotice) public returns (bytes32) {
        return getParentHash(theNotice);
    }

    function hashNoticeExt(bytes calldata theNotice) public returns (bytes32) {
        return hashNotice(theNotice);
    }

    function hashNotice(bytes calldata data) override internal pure returns (bytes32) {
        return keccak256(data[4:68]);
    }

    function checkNoticeChain(bytes32 targetHash) public {
        bytes[] memory notices = new bytes[](havocdNotices.length);
        for (uint i = 0; i < havocdNotices.length; i++) {
            notices[i] = havocdNotices[i];
        }
        this.checkNoticeChainAuthorizedExt(targetHash, notices);
    }

    function checkNoticeChainAuthorizedExt(
        bytes32 targetHash,
        bytes[] calldata notices
    ) public {
        checkNoticeChainAuthorized(targetHash, notices);
    }

    function recover(bytes32 digest, bytes memory signature) override internal view returns (address) {
        return recoverer[digest][signature];
    }
}