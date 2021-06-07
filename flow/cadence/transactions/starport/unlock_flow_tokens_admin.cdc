// Update new authorities addresses
import Starport from 0xc8873a26b148ed14

transaction(toAddress: Address, amount: UFix64) {
     let admin: &Starport.Administrator

    prepare(signer: AuthAccount) {
        self.admin = signer.borrow<&Starport.Administrator>(from: /storage/admin)
            ?? panic("Could not borrow reference to storage Starport Participant")
    }

    execute {
        self.admin.unlock(toAddress: toAddress, amount: amount)
    }
}