// SPDX-License-Identifier: GPL-3.0
pragma solidity ^0.8.1;
pragma abicoder v2;

import "./ICash.sol";

/**
 * @title Compound Chain Starport
 * @author Compound Finance
 * @notice Contract to link Ethereum to Compound Chain
 * @dev XXX Many TODOs
 */
contract Starport {
    ICash immutable public cash;

    address constant public ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;
    bytes4 constant MAGIC_HEADER = "ETH:";
    address[] public authorities;

    uint public eraId; // TODO: could bitpack here and use uint32
    mapping(bytes32 => bool) public isNoticeUsed;

    event Lock(IERC20 asset, address holder, uint amount);
    event LockCash(address holder, uint amount, uint yieldIndex);
    event Unlock(address account, uint amount, IERC20 asset);
    event ChangeAuthorities(address[] newAuthorities);

    constructor(ICash cash_, address[] memory authorities_) {
        cash = cash_;
        authorities = authorities_;

        emit ChangeAuthorities(authorities_);
    }

    /**
     * Section: Ethereum Asset Interface
     */

    /**
     * @notice Transfer an asset to Compound Chain via locking it in the Starport
     * @dev Use `lockEth` to lock Ether. Note: locking CASH will burn the CASH from Ethereum.
     * @param amount The amount (in the asset's native wei) to lock
     * @param asset The asset to lock in the Starport
     */
    function lock(uint amount, IERC20 asset) public {
        // TODO: Check Supply Cap
        if (asset == cash) {
            lockCashInternal(amount, msg.sender);
        } else {
            lockInternal(amount, asset, msg.sender);
        }
    }

    /*
     * @notice Transfer Eth to Compound Chain via locking it in the Starport
     * @dev Use `lock` to lock CASH or collateral assets.
     */
    function lockEth() public payable {
        // TODO: Check Supply Cap
        emit Lock(IERC20(ETH_ADDRESS), msg.sender, msg.value);
    }

    // Internal function for locking CASH (as opposed to collateral assets)
    function lockCashInternal(uint amount, address sender) internal {
        // cash.burn(amount);
        uint yieldIndex = cash.getCashIndex();
        transferInCash(sender, amount);

        emit LockCash(sender, amount, yieldIndex);
    }

    // Internal function for locking non-ETH collateral assets
    function lockInternal(uint amount, IERC20 asset, address sender) internal {
        // TODO: Check Supply Cap
        uint amountTransferred = transferIn(sender, amount, asset);

        emit Lock(asset, sender, amountTransferred);
    }

    // Transfer in an asset, returning the balance actually accrued (i.e. less token fees)
    // Note: do not use for Ether or CASH (XXX: Why not CASH?)
    function transferIn(address from, uint amount, IERC20 asset) internal returns (uint) {
        uint balanceBefore = asset.balanceOf(address(this));
        INonStandardERC20(address(asset)).transferFrom(from, address(this), amount);

        bool success;
        assembly {
            switch returndatasize()
                case 0 {                       // This is a non-standard ERC-20
                    success := not(0)          // set success to true
                }
                case 32 {                      // This is a compliant ERC-20
                    returndatacopy(0, 0, 32)
                    success := mload(0)        // Set `success = returndata` of external call
                }
                default {                      // This is an excessively non-compliant ERC-20, revert.
                    revert(0, 0)
                }
        }
        require(success, "transferIn failed");

        uint balanceAfter = asset.balanceOf(address(this));
        return balanceAfter - balanceBefore;
    }

    // Transfer out an asset
    // Note: we do not check fees here, since we do not account for them
    function transferOut(address to, uint amount, IERC20 asset) internal {
        INonStandardERC20(address(asset)).transfer(to, amount);

        bool success;
        assembly {
            switch returndatasize()
                case 0 {                       // This is a non-standard ERC-20
                    success := not(0)          // set success to true
                }
                case 32 {                      // This is a complaint ERC-20
                    returndatacopy(0, 0, 32)
                    success := mload(0)        // Set `success = returndata` of external call
                }
                default {                      // This is an excessively non-compliant ERC-20, revert.
                    revert(0, 0)
                }
        }
        require(success, "transferOut failed");
    }

    // TODO: Why not just use `transferIn`?
    function transferInCash(address from, uint amount) internal {
        require(cash.transferFrom(from, address(this), amount) == true, "TransferInCash");
    }

    /*
     * @notice Transfer Eth to Compound Chain via locking it in the Starport
     * @dev This is a shortcut for `lockEth`. See `lockEth` for more details.
     */
    receive() external payable {
        lockEth();
    }

    /**
     * Section: L2 Message Ports
     **/

    /**
     * @notice Invoke a signed notice from the Starport, which will execute a function such as unlock.
     * @dev Notices are generated by certain actions from Compound Chain and signed by validators.
     * @param notice The notice generated by Compound Chain, encoded for Ethereum.
     * @param signatures Signatures from a quorum of validator nodes from Compound Chain.
     * @return The result of the invokation of the action of the notice.
     */
    function invoke(bytes calldata notice, bytes[] calldata signatures) external returns (bytes memory) {
        checkNoticeAuthorized(notice, authorities, signatures);

        return invokeNoticeInternal(notice);
    }

    // Invoke without authorization checks used by external functions
    function invokeNoticeInternal(bytes calldata notice) internal returns (bytes memory) {
        // XXX Really consider what to do with eraId, eraIndex and parent
        // TODO: By hash or by (eraId, eraIndex)?
        // TODO: Should eraId, eraIndex and parent be handled specially?
        isNoticeUsed[hashNotice(notice)] = true;

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

        bytes memory calldata_ = bytes(notice[100:]);
        (bool success, bytes memory callResult) = address(this).call(calldata_);
        require(success, "Call failed");

        return callResult;
    }

    /**
     * @notice Unlock the given asset from the Starport
     * @dev This must be called from `invoke` via passing in a signed notice from Compound Chain.
     * @param asset The Asset to unlock
     * @param amount The amount of the asset to unlock in its native token units
     * @param account The account to transfer the asset to
     */
    function unlock(IERC20 asset, uint amount, address account) external {
        require(msg.sender == address(this), "Call must originate locally");

        // XXX TODO: This needs to handle Ether, Cash and collateral tokens
        emit Unlock(account, amount, asset);

        transferOut(account, amount, asset);
    }

    /**
     * @notice Rotates authorities which can be used to sign notices for the Staport
     * @dev This must be called from `invoke` via passing in a signed notice from Compound Chain.
     * @param newAuthorities The new authorities which may sign notices for execution by the Starport
     */
    function changeAuthorities(address[] calldata newAuthorities) external {
        require(msg.sender == address(this), "Call must originate locally");
        require(newAuthorities.length > 0, "New authority set can not be empty");

        // XXX TODO: min authorities length?
        // XXX TODO: check for repeats in the authorities list?

        emit ChangeAuthorities(newAuthorities);

        authorities = newAuthorities;
    }

    /**
     * Section: View Helpers
     */

    /**
     * @notice Returns the current authority nodes
     * @return The current authority node addresses
     */
    function getAuthorities() public view returns (address[] memory) {
        return authorities;
    }

    /**
     * @notice Checks that the given notice is authorized
     * @dev Notices are authorized by having a quorum of signatures from the `authorities` set
     * @dev Notices can be separately validated by a notice chain XXX TODO
     * @dev Reverts if notice is not authorized XXX TODO: Is even useful then?
     * @param notice The notice to verify authenticity of
     * @param authorities_ A set of authorities to check the notice against? TODO: Why pass this in?
     * @param signatures The signatures to verify
     */
    function checkNoticeAuthorized(
        bytes calldata notice,
        address[] memory authorities_,
        bytes[] calldata signatures
    ) internal view {
        bytes32 noticeHash = hashNotice(notice);
        require(isNoticeUsed[noticeHash] == false, "Notice can not be reused");

        address[] memory sigs = new address[](signatures.length);
        for (uint i = 0; i < signatures.length; i++) {
            address signer = recover(noticeHash, signatures[i]);
            require(contains(sigs, signer) == false, "Duplicated authority signer");
            require(contains(authorities_, signer) == true, "Unauthorized authority signer");
            sigs[i] = signer;
        }

        require(sigs.length >= getQuorum(authorities_.length), "Below quorum threshold");
    }

    /**
     * Section: Pure Function Helpers
     */

    // Helper function to hash a notice
    function hashNotice(bytes calldata data) internal pure returns (bytes32) {
        return keccak256((abi.encodePacked(data)));
    }

    // Helper function to check if a given list contains an element
    function contains(address[] memory arr, address elem) internal pure returns (bool) {
        for (uint i = 0; i < arr.length; i++) {
            if (arr[i] == elem) {
                return true;
            }
        }
        return false;
    }

    // Quorum is >1/3 authorities approving (XXX TODO: 1/3??)
    function getQuorum(uint authorityCount) internal pure returns (uint) {
        return (authorityCount / 3) + 1;
    }

    // Adapted from https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/cryptography/ECDSA.sol
    function recover(bytes32 digest, bytes memory signature) internal pure returns (address) {
        // Check the signature length
        if (signature.length != 65) {
            revert("ECDSA: invalid signature length");
        }

        // Divide the signature in r, s and v variables
        bytes32 r;
        bytes32 s;
        uint8 v;

        // ecrecover takes the signature parameters, and the only way to get them
        // currently is to use assembly.
        // solhint-disable-next-line no-inline-assembly
        assembly {
            r := mload(add(signature, 0x20))
            s := mload(add(signature, 0x40))
            v := byte(0, mload(add(signature, 0x60)))
        }

        // XXX Does this mean EIP-155 signatures are considered invalid?
        require(v == 27 || v == 28, "ECDSA: invalid signature 'v' value");

        // If the signature is valid (and not malleable), return the signer address
        address signer = ecrecover(digest, v, r, s);
        require(signer != address(0), "ECDSA: invalid signature");

        return signer;
    }
}
