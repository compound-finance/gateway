// Withdraw Locked Flow Tokens
//import FlowToken from 0x0ae53cb6e3f42a79
import FlowToken from 0x7e60df042a9c0868
import Starport from 0xc8873a26b148ed14

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