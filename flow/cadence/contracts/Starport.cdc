// import FlowToken from 0x0ae53cb6e3f42a79
// import FungibleToken from 0xee82856bf20e2aa6

import FungibleToken from 0x9a0766d93b6608b7
import FlowToken from 0x7e60df042a9c0868

import Crypto

pub contract Starport {

    // Event that is emitted when Flow tokens are locked in the Starport vault
    //event Lock(address indexed asset, address indexed sender, string chain, bytes32 indexed recipient, uint amount);
    pub event Lock(asset: String, recipient: Address?, amount: UFix64)

    // Event that is emitted when tokens are unlocked by Gateway
    pub event Unlock(account: Address, amount: UFix64, asset: String)

    // Event that is emitted when a set of authorities is changed
    pub event ChangeAuthorities(newAuthorities: [String])

    // Event that is emitted when new supply cap for an asset is set
    pub event NewSupplyCap(asset: String, supplyCap: UFix64)

    // Event that is emitted when locked Flow tokens are withdrawn
    pub event TokensWithdrawn(asset: String, amount: UFix64)

    // Event that is emitted when notice is incorrect
    pub event NoticeError(noticeEraId: UInt256, noticeEraIndex: UInt256, error: String)

    // Private vault with public deposit function
    access(self) var vault: @FlowToken.Vault

    // TODO check if there is a way to store Starport keylist
    access(self) var authorities: [String]

    access(self) var eraId: UInt256

    access(self) var invokedNotices: [String]

    access(self) var unlockHex: String

    access(self) var changeAuthoritiesHex: String

    access(self) var setSupplyCapHex: String

    access(self) var flowAssetHex: String

    access(self) var supplyCaps: {String: UFix64}

    /// This interface for locking Flow tokens.
    pub resource interface FlowLock {
        pub fun lock(from: @FungibleToken.Vault)
    }

    pub resource StarportParticipant: FlowLock {

        pub fun lock(from: @FungibleToken.Vault) {
            pre {
                // Starport.vault.balance <= Starport.supplyCaps["FLOW"]: "Supply Cap Exceeded"
                Starport.vault.balance + from.balance <= Starport.getFlowSupplyCap(): "Supply Cap Exceeded"
            }
            let from <- from as! @FlowToken.Vault
            let balance = from.balance
            Starport.vault.deposit(from: <-from)
            emit Lock(asset: "FLOW", recipient: self.owner?.address, amount: balance);
        }
    }

    pub resource Administrator {
        // withdraw
        // TODO - delete it later?
        // Allows the administrator to withdraw locked Flow tokens
        pub fun withdrawLockedFlowTokens(amount: UFix64): @FungibleToken.Vault {
            let vault <- Starport.vault.withdraw(amount: amount)
            emit TokensWithdrawn(asset: "FLOW", amount: amount)
            return <-vault
        }

        // Unlock
        //
        // Allows the administrator to unlock Flow tokens
        pub fun unlock(toAddress: Address, amount: UFix64) {
            // Get capability to deposit tokens to `toAddress` receiver
            let toAddressReceiver = getAccount(toAddress)
                .getCapability<&FlowToken.Vault{FungibleToken.Receiver}>(/public/flowTokenReceiver)!
                .borrow() ?? panic("Could not borrow FLOW receiver capability")

            // Withdraw Flow tokens to the temporary vault
            let temporaryVault <- Starport.vault.withdraw(amount: amount)

            // Deposit Flow tokens to receiver from the temporary vault
            toAddressReceiver.deposit(from: <-temporaryVault)

            // Emit event
            emit Unlock(account: toAddress, amount: amount, asset: "FLOW")
        }

        pub fun changeAuthorities(newAuthorities: [String]) {
            //TODO post checks here?
            pre {
                newAuthorities.length > 0: "New authority set can not be empty"
            }
            Starport.authorities = newAuthorities

            emit ChangeAuthorities(newAuthorities: newAuthorities)
        }

        pub fun setSupplyCap(supplyCap: UFix64) {
            emit NewSupplyCap(asset: "FLOW", supplyCap: supplyCap);

            Starport.supplyCaps["FLOW"] = supplyCap;
        }
    }

    pub fun createStarportParticipant(): @StarportParticipant {
        return <- create StarportParticipant()
    }

    pub fun getLockedBalance(): UFix64 {
        return self.vault.balance
    }

    pub fun getAuthorities(): [String] {
        return self.authorities
    }

    pub fun getEraId(): UInt256 {
        return self.eraId
    }

    pub fun getFlowSupplyCap(): UFix64 {
        return self.supplyCaps["FLOW"] ?? UFix64(0)
    }

    // Unlock Flow tokens with notice
    pub fun unlock(noticeEraId: UInt256,
                   noticeEraIndex: UInt256,
                   parentNoticeHex: String,
                   toAddress: Address,
                   amount: UFix64,
                   signatures: [String]) {
        // Build `unlock` notice message
        let message = self.buildUnlockMessage(noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            parentNoticeHex: parentNoticeHex,
            toAddress: toAddress,
            amount: amount)

        // Validate notice's signatures, invocation status and era ids
        if !self.isValidNotice(noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            message: message,
            signatures: signatures) {
            log("Invalid Notice")
            return
        }

        // Get capability to deposit tokens to `toAddress` receiver
        let toAddressReceiver = getAccount(toAddress)
            .getCapability<&FlowToken.Vault{FungibleToken.Receiver}>(/public/flowTokenReceiver)!
            .borrow() ?? panic("Could not borrow FLOW receiver capability")

        // Withdraw Flow tokens to the temporary vault
        let temporaryVault <- Starport.vault.withdraw(amount: amount)
        // TODO - do we need this event?
        emit TokensWithdrawn(asset: "FLOW", amount: amount)

        // Deposit Flow tokens to receiver from the temporary vault
        toAddressReceiver.deposit(from: <-temporaryVault)

        // Emit event
        emit Unlock(account: toAddress, amount: amount, asset: "FLOW")
    }

    pub fun buildUnlockMessage(noticeEraId: UInt256,
                               noticeEraIndex: UInt256,
                               parentNoticeHex: String,
                               toAddress: Address,
                               amount: UFix64): [UInt8] {
        let message = self.unlockHex.decodeHex()
               .concat(noticeEraId.toBigEndianBytes())
               .concat(noticeEraIndex.toBigEndianBytes())
               .concat(parentNoticeHex.decodeHex())
               .concat(toAddress.toBytes())
               .concat(amount.toBigEndianBytes())
        return message
    }

    access(self) fun buildNoticeHash(noticeEraId: UInt256, noticeEraIndex: UInt256): String {
        return noticeEraId.toString().concat("_").concat(noticeEraIndex.toString())
    }

    access(self) fun checkNoticeSignerAuthorized(message: [UInt8], signatures: [String]): Bool {
        let authoritiesList: [Crypto.KeyList] = []

        for authority in Starport.authorities {
            let keylist = Crypto.KeyList()
            let publicKey = PublicKey(
                publicKey: authority.decodeHex(),
                signatureAlgorithm: SignatureAlgorithm.ECDSA_Secp256k1
            )

            keylist.add(
                publicKey,
                hashAlgorithm: HashAlgorithm.SHA3_256,
                weight: 1.0 // Check weight, think about value here
            )
            authoritiesList.append(keylist)
        }

        let validSigs: [Int] = []
        for signature in signatures {
            let signatureSet: [Crypto.KeyListSignature] = []
            signatureSet.append(
                Crypto.KeyListSignature(
                    keyIndex: 0,
                    signature: signature.decodeHex()
                )
            )

            var i = 0
            for keyList in authoritiesList {
                if keyList.isValid(signatureSet: signatureSet, signedData: message) {
                    if !validSigs.contains(i) {
                        validSigs.append(i)
                        break
                    }
                }
                i = i + 1
            }
        }

        return validSigs.length >= self.getQuorum(authorityCount: self.authorities.length)
    }

    access(self) fun getQuorum(authorityCount: Int): Int {
        return (authorityCount / 3) + 1
    }

    // Validates notice's signatures, invocation status and era ids
    // @note Side effects: notice invocation status and eraId storage values can be changed
    access(self) fun isValidNotice(noticeEraId: UInt256,
                                   noticeEraIndex: UInt256,
                                   message: [UInt8],
                                   signatures: [String]): Bool {
        // Check validity of signatures for the notice
        let isSigValid = self.checkNoticeSignerAuthorized(message: message, signatures: signatures)
        if !isSigValid {
            log("Error unlocking Flow tokens, signatures are incorrect")
            emit NoticeError(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex, error: "Signatures are incorrect")
            return false
        }

        // Check invocation status of the notice
        let noticeHash = self.buildNoticeHash(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex)
        if self.invokedNotices.contains(noticeHash) {
            log("Notice replay")
            emit NoticeError(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex, error: "Notice replay")
            return false
        }
        // Mark notice as being invoked
        self.invokedNotices.append(noticeHash)

        // Check that notice has a correct era
        let startNextEra: Bool = noticeEraId == self.eraId + UInt256(1) && noticeEraIndex == UInt256(0)
        if !(noticeEraId <= self.eraId || startNextEra) {
            log("Notice must use existing era or start next era")
            emit NoticeError(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex, error: "Notice must use existing era or start next era")
            return false
        }
        // Update era
        if startNextEra {
            self.eraId = self.eraId + UInt256(1);
        }

        return true
    }

    pub fun changeAuthorities(noticeEraId: UInt256,
                              noticeEraIndex: UInt256,
                              parentNoticeHex: String,
                              newAuthorities: [String],
                              signatures: [String]) {
        pre {
            newAuthorities.length > 0: "New authority set can not be empty"
        }

        // Build `changeAuthorities` notice message
        let message = self.buildChangeAuthoritiesMessage(
                noticeEraId: noticeEraId,
                noticeEraIndex: noticeEraIndex,
                parentNoticeHex: parentNoticeHex,
                newAuthorities: newAuthorities)

        // Validate notice's signatures, invocation status and era ids
        if !self.isValidNotice(noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            message: message,
            signatures: signatures) {
            log("Invalid Notice")
            return
        }

        Starport.authorities = newAuthorities

        emit ChangeAuthorities(newAuthorities: newAuthorities)
    }

    pub fun buildChangeAuthoritiesMessage(noticeEraId: UInt256,
                                          noticeEraIndex: UInt256,
                                          parentNoticeHex: String,
                                          newAuthorities: [String]): [UInt8] {
            var message = self.changeAuthoritiesHex.decodeHex()
                .concat(noticeEraId.toBigEndianBytes())
                .concat(noticeEraIndex.toBigEndianBytes())
                .concat(parentNoticeHex.decodeHex())

            for newAuthority in newAuthorities {
                message = message.concat(newAuthority.decodeHex())
            }

            return message
    }

    pub fun setSupplyCap(noticeEraId: UInt256,
                         noticeEraIndex: UInt256,
                         parentNoticeHex: String,
                         supplyCap: UFix64,
                         signatures: [String]) {
        // Build `setSupplyCap` notice message
        let message = self.buildSetSupplyCapMessage(
                noticeEraId: noticeEraId,
                noticeEraIndex: noticeEraIndex,
                parentNoticeHex: parentNoticeHex,
                supplyCap: supplyCap)

        // Validate notice's signatures, invocation status and era ids
        if !self.isValidNotice(noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            message: message,
            signatures: signatures) {
            log("Invalid Notice")
            return
        }

        emit NewSupplyCap(asset: "FLOW", supplyCap: supplyCap);

        Starport.supplyCaps["FLOW"] = supplyCap;
    }

    pub fun buildSetSupplyCapMessage(noticeEraId: UInt256,
                                     noticeEraIndex: UInt256,
                                     parentNoticeHex: String,
                                     supplyCap: UFix64): [UInt8] {
        let message = self.setSupplyCapHex.decodeHex()
               .concat(noticeEraId.toBigEndianBytes())
               .concat(noticeEraIndex.toBigEndianBytes())
               .concat(parentNoticeHex.decodeHex())
               .concat(self.flowAssetHex.decodeHex())
               .concat(supplyCap.toBigEndianBytes())
        return message
    }

    init() {
        // Create a new Starport Vault and save it in storage
        self.vault <- FlowToken.createEmptyVault() as! @FlowToken.Vault

        // Create a new Admin resource
        let admin <- create Administrator()
        self.account.save(<-admin, to: /storage/admin)

        // Set intitial values
        self.authorities = []
        self.eraId = UInt256(0)
        self.invokedNotices = []
        self.supplyCaps = {}
        // `unlock` method name hex encoded
        self.unlockHex = "756e6c6f636b"
        // `changeAuthorities` method name hex encoded
        self.changeAuthoritiesHex = "6368616e6765417574686f726974696573"
        self.setSupplyCapHex = "736574537570706c79436170"
        // `FLOW` asset name in hex
        self.flowAssetHex = "464c4f57"
    }
}