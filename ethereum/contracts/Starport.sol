pragma solidity ^0.7.5;
pragma abicoder v2;

import "./IERC20.sol";

// via https://github.com/dapphub/ds-math/blob/master/src/math.sol
function add_(uint x, uint y) pure returns (uint z) {
    require((z = x + y) >= x, "ds-math-add-overflow");
}

function sub_(uint x, uint y) pure returns (uint z) {
    require((z = x - y) <= x, "ds-math-sub-underflow");
}

function mul_(uint x, uint y) pure returns (uint z) {
    require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
}

contract Starport {

	ICash immutable public cash;

	address constant public ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;
	bytes32 constant public ETH_CHAIN_IDENT = keccak256(abi.encodePacked("ETH"));
	byte constant MAGIC_HEADER_0 = 0x45;
	byte constant MAGIC_HEADER_1 = 0x54;
	byte constant MAGIC_HEADER_2 = 0x48;
	byte constant MAGIC_HEADER_3 = 0x3a;
	uint constant HEAD_BYTES = 99; // bytes3 chainIdent, uint256 eraId, uint256 eraIndex, bytes32 parent
	address[] public authorities;

	uint public eraId; // TODO: could bitpack here and use uint32
	mapping(bytes32 => bool) public isNoticeUsed;

	event LockCash(address holder, uint amount, uint yieldIndex);
	event Lock(address asset, address holder, uint amount);
	event Unlock(address account, uint amount, address asset);
	event ChangeAuthorities(bytes32 authHash);

	constructor(ICash cash_, address[] memory authorities_) {
		cash = cash_;
		authorities = authorities_;
	}

	// ** L1 Asset Interface **

	function lock(uint amount, address asset) public {
		if (asset == address(cash)) {
			lockCashInternal(amount, msg.sender);
		} else {
			lockInternal(amount, asset, msg.sender);
		}
	}

	function lockETH() public payable {
		// TODO: Check Supply Cap
		emit Lock(ETH_ADDRESS, msg.sender, msg.value);
	}

	function lockCashInternal(uint amount, address sender) internal {
		// cash.burn(amount);
		uint yieldIndex = cash.fetchHypotheticalIndex();
		transferInCash(sender, amount);
		emit LockCash(sender, amount, yieldIndex);
	}

	function lockInternal(uint amount, address asset, address sender) internal {
		// TODO: Check Supply Cap
		uint amountTransferred = transferIn(sender, amount, IERC20(asset));
		emit Lock(asset, sender, amountTransferred);
	}

	// Make sure that the amount we ask for
	function transferIn(address from, uint amount, IERC20 asset) internal returns (uint) {
		uint balBefore = asset.balanceOf(address(this));
		require(asset.transferFrom(from, address(this), amount) == true, "TransferIn"); // TODO: Handle non-standard tokens
		uint balAfter = asset.balanceOf(address(this));
		return sub_(balAfter, balBefore);
	}

	function transferInCash(address from, uint amount) internal {
		require(cash.transferFrom(from, address(this), amount) == true, "TransferInCash");
	}

	receive() external payable {
		lockETH();
	}

	// ** L2 Message Ports **

	// TODO: Really consider what to do with eraId, eraIndex and parent
	function invoke(bytes calldata notice, bytes[] calldata signatures) external returns (bytes memory) {
		assertNoticeAuthorized(notice, authorities, signatures, false); // TODO: Clean up signature

		// TODO: By hash or by (eraId, eraIndex)?
		// TODO: Should eraId, eraIndex and parent be handled specially?
		isNoticeUsed[hash(notice)] = true;

		require(notice.length >= 100, "Must have full header"); // 4 + 3 * 32
		require(notice[0] == MAGIC_HEADER_0, "Invalid header[0]");
		require(notice[1] == MAGIC_HEADER_1, "Invalid header[1]");
		require(notice[2] == MAGIC_HEADER_2, "Invalid header[2]");
		require(notice[3] == MAGIC_HEADER_3, "Invalid header[3]");
		(uint noticeEraId, uint noticeEraIndex, bytes32 noticeParent) =
			abi.decode(notice[4:100], (uint, uint, bytes32));

		(noticeEraId, noticeEraIndex, noticeParent); // unused

		require(noticeEraId == eraId, "Notice must use current era"); // TODO: Admin notice

		bytes memory calldata_ = bytes(notice[100:]);
		(bool success, bytes memory callResult) = address(this).call(calldata_);
		require(success, "Call failed");

		return callResult;
	}

	function unlock(address asset, uint amount, address account) external {
		require(msg.sender == address(this), "Call must originate locally");

		emit Unlock(account, amount, asset);

		IERC20(asset).transfer(account, amount);
	}

	// @dev notice = (bytes3 chainIdent, uint256 eraId, uint256 eraIndex, address[] newAuths)
	function changeAuthorities(bytes calldata notice, bytes[] calldata signatures) external {
		require(notice.length >= 99, "New authority set can not be empty");//99 bytes of header, 32 * n bytes of auths
		assertNoticeAuthorized(notice, authorities, signatures, true);

		bytes calldata body = notice[HEAD_BYTES:];
		require(body.length % 32 == 0, "Excess bytes");
		uint numAuths = body.length / 32;// evm word size is 32 bytes

		// Decode the notice into a new auth array
		address[] memory newAuths = new address[](numAuths);
		for (uint i = 0; i < numAuths; i ++) {
			uint startIdx = mul_(i, 32);
			uint endIdx = add_(startIdx, 32);
			address newAuth = abi.decode(body[startIdx:endIdx], (address));
			newAuths[i] = newAuth;
		}
		bytes32 authHash = hash(newAuths);
		emit ChangeAuthorities(authHash);
		authorities = newAuths;
		isNoticeUsed[hash(notice)] = true;
		eraId = add_(eraId, 1);
	}


	// ** VIEW HELPERS **

	function getAuthorities() public view returns (address[] memory){
		return authorities;
	}

	// @dev Reverts if notice is not authorized
	// * the first 7 bytes of a notice is always {bytes3 chainIdent, uint256 eraId}
	function assertNoticeAuthorized(
		bytes calldata message,
		address[] memory authorities_,
		bytes[] calldata signatures,
		bool isAdminNotice
	) public view {
		bytes32 noticeHash = hash(message);
		require(isNoticeUsed[noticeHash] == false, "Notice can not be reused");

		address[] memory sigs = new address[](signatures.length);
		for (uint i = 0; i < signatures.length; i++) {
			address signer = recover(noticeHash, signatures[i]);
			require(contains(sigs, signer) == false, "Duplicated sig");
			require(contains(authorities_, signer) == true, "Unauthorized signer");
			sigs[i] = signer;
		}
		require(sigs.length >= getQuorum(authorities_.length), "Below quorum threshold");
	}

	// ** PURE HELPERS **

	function hash(address[] memory data) public pure returns (bytes32) {
		return keccak256((abi.encodePacked(data)));
	}

	function hash(bytes calldata data) public pure returns (bytes32) {
		return keccak256((abi.encodePacked(data)));
	}

	function contains(address[] memory arr, address elem) internal pure returns (bool) {
		for(uint i = 0; i < arr.length; i++) {
			if (arr[i] == elem) return true;
		}
		return false;
	}

	// @dev Quorum is > 1/3 authorities approving
	function getQuorum(uint authorityCount) public pure returns (uint) {
		return (authorityCount / 3) + 1;
	}


	// @dev Adapted from https://github.com/OpenZeppelin/openzeppelin-contracts/blob/master/contracts/cryptography/ECDSA.sol
	function recover(bytes32 hashedMsg, bytes memory signature) public pure returns (address) {
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

	    require(v == 27 || v == 28, "ECDSA: invalid signature 'v' value");

	    // If the signature is valid (and not malleable), return the signer address
	    address signer = ecrecover(hashedMsg, v, r, s);
	    require(signer != address(0), "ECDSA: invalid signature");

	    return signer;
	}
}
