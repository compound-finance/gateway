pragma solidity ^0.7.5;
pragma abicoder v2;

import "./IERC20.sol";

// via https://github.com/dapphub/ds-math/blob/master/src/math.sol
function add(uint x, uint y) pure returns (uint z) {
    require((z = x + y) >= x, "ds-math-add-overflow");
}

function sub(uint x, uint y) pure returns (uint z) {
    require((z = x - y) <= x, "ds-math-sub-underflow");
}

function mul(uint x, uint y) pure returns (uint z) {
    require(y == 0 || (z = x * y) / y == x, "ds-math-mul-overflow");
}

contract Starport {

	ICash immutable public cash;

	bytes32 immutable public CHAIN_TYPE_ETH = keccak256(abi.encodePacked("ETH"));
	address immutable public ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;
	address[] public authorities;

	event LockCash(address holder, uint amount, uint yieldIndex);
	event Lock(address asset, address holder, uint amount);

	constructor(ICash cash_, address[] memory authorities_) {
		cash = cash_;
		authorities = authorities_;
	}

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
		require(asset.transferFrom(from, address(this), amount) == true, "TransferIn");
		uint balAfter = asset.balanceOf(address(this));
		return sub(balAfter, balBefore);
	}

	function transferInCash(address from, uint amount) internal {
		require(cash.transferFrom(from, address(this), amount) == true, "TransferInCash");
	}

	// ** MESSAGES **
	function changeAuthorities(bytes calldata changeAuthoritiesNotice, bytes[] calldata signatures) public {
		// TODO: address[] memory authTemp = authorities;
		// isMsgAuthorized(changeAuthoritiesNotice, authTemp, signatures);
		verifyChainType(changeAuthoritiesNotice);

		uint CHAIN_TYPE_BYTES = 3;
		uint WORD_SIZE = 32;

		uint len = sub(changeAuthoritiesNotice.length, CHAIN_TYPE_BYTES);
		require(len % WORD_SIZE == 0, "Excess bytes");
		uint numAuths = len / WORD_SIZE;

		address[] memory newAuths = new address[](numAuths);
		for (uint i = 0; i < numAuths; i ++) {
			uint startIdx = add(CHAIN_TYPE_BYTES, mul(i, WORD_SIZE));
			uint endIdx = add(startIdx, WORD_SIZE);
			address newAuth = abi.decode(changeAuthoritiesNotice[startIdx:endIdx], (address));
			newAuths[i] = newAuth;
		}
		authorities = newAuths;
		// TODO: Let authHash = hash(newAuthorities)
		// Emit ChangeAuthorities(authHash: bytes32)
	}


	function verifyChainType(bytes calldata notice) public view returns (bool) {
		//ABI Decoding requires padded values, so we have to grab bytes3 manually and pass the full memory slot for addresses
		// https://solidity.readthedocs.io/en/latest/types.html?highlight=slice#array-slices
		bytes3 chainType =
			notice[0] |
            (bytes3(notice[1]) >> 8) |
            (bytes3(notice[2]) >> 16);

        require(keccak256(abi.encodePacked(chainType)) == CHAIN_TYPE_ETH, "Invalid Sig");
	}

	function getAuthorities() public view returns (address[] memory){
		return authorities;
	}

	// ** PURE HELPERS **


	// @dev Reverts if message is not authorized
	function isMsgAuthorized(
		bytes calldata message,
		address[] memory authorities_,
		bytes[] memory signatures
	) public pure returns (bool) {
		address[] memory sigs = new address[](signatures.length);
		for (uint i = 0; i < signatures.length; i++) {
			address signer = recover(keccak256(message), signatures[i]);
			require(contains(sigs, signer) == false, "Duplicated sig");
			require(contains(authorities_, signer) == true, "Unauthorized signer");
			sigs[i] = signer;
		}
		require(sigs.length >= getQuorum(authorities_.length), "Below quorum threshold");
		return true;
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
	function recover(bytes32 hash, bytes memory signature) public pure returns (address) {
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
	    address signer = ecrecover(hash, v, r, s);
	    require(signer != address(0), "ECDSA: invalid signature");

	    return signer;
	}

	receive() external payable {
		lockETH();
	}
}
