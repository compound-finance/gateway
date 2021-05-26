// Lock Flow Tokens
import FlowToken from 0x0ae53cb6e3f42a79
import FungibleToken from 0xee82856bf20e2aa6
import Starport from 0xf8d6e0586b0a20c7

transaction(lockAmount: UFix64) {
    // let tokenAdmin: &FlowToken.Administrator
    let participant: &{Starport.FlowLock}

    // The Vault resource that holds the tokens that are being transferred
    let sentVault: @FungibleToken.Vault

    prepare(signer: AuthAccount) {
        // Get a reference to the signer's stored vault
        let vaultRef = signer.borrow<&FlowToken.Vault>(from: /storage/flowTokenVault)
			?? panic("Could not borrow reference to the owner's Vault!")

        // Withdraw tokens from the signer's stored vault
        self.sentVault <- vaultRef.withdraw(amount: lockAmount)

        // Get an access to Starport Participant for locking Flow tokens
        self.participant = signer
            .getCapability(/public/participant)
            .borrow<&{Starport.FlowLock}>()
             ?? panic("Could not borrow Starport participant")
    }

    execute {
        // Lock tokens in Starport
        self.participant.lock(from: <-self.sentVault)
    }
}