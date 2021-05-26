// Withdraw Locked Flow Tokens
import FlowToken from 0x0ae53cb6e3f42a79
import Starport from 0xf8d6e0586b0a20c7

transaction(lockedAmount: UFix64) {
    let admin: &Starport.Administrator

    prepare(signer: AuthAccount) {

        self.admin = signer.borrow<&Starport.Administrator>(from: /storage/admin)
            ?? panic("Could not borrow reference to storage Starport Participant")
    }

    execute {

        let vault <- self.admin.withdrawLockedFlowTokens(amount: lockedAmount)
        destroy vault

    }
}