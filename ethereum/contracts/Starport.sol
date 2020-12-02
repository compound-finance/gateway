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

	bytes32 immutable public ETH_CHAIN_TYPE = keccak256(abi.encodePacked("ETH"));
	address immutable public ETH_ADDRESS = 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE;
	address[] public authorities;

	event LockCash(address holder, uint amount, uint yieldIndex);
	event Lock(address asset, address holder, uint amount);
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
		require(asset.transferFrom(from, address(this), amount) == true, "TransferIn");
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
	function changeAuthorities(bytes calldata notice, bytes[] calldata signatures) public {
		require(notice.length >= 35, "New authority set can not be empty");
		bytes calldata chainType = notice[0:3]; // first 3 bytes of notice are chain type
  		require(keccak256(abi.encodePacked(chainType)) == ETH_CHAIN_TYPE, "Invalid Chain Type");
		bytes calldata body = notice[3:];
		require(body.length % 32 == 0, "Excess bytes");

		isMsgAuthorized(notice, authorities, signatures);
		uint numAuths = body.length / 32;// evm word size is 32 bytes

		address[] memory newAuths = new address[](numAuths);
		for (uint i = 0; i < numAuths; i ++) {
			uint startIdx = mul_(i, 32);
			uint endIdx = add_(startIdx, 32);
			address newAuth = abi.decode(body[startIdx:endIdx], (address));
			newAuths[i] = newAuth;
		}
		bytes32 authHash = keccak256(abi.encodePacked(newAuths));
		emit ChangeAuthorities(authHash);
		authorities = newAuths;
	}


	// ** VIEW HELPERS **

	function getAuthorities() public view returns (address[] memory){
		return authorities;
	}

	// ** PURE HELPERS **


	// @dev Reverts if message is not authorized
	function isMsgAuthorized(
		bytes calldata message,
		address[] memory authorities_,
		bytes[] calldata signatures
	) public pure {
		address[] memory sigs = new address[](signatures.length);
		for (uint i = 0; i < signatures.length; i++) {
			address signer = recover(keccak256(message), signatures[i]);
			require(contains(sigs, signer) == false, "Duplicated sig");
			require(contains(authorities_, signer) == true, "Unauthorized signer");
			sigs[i] = signer;
		}
		require(sigs.length >= getQuorum(authorities_.length), "Below quorum threshold");
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
}
