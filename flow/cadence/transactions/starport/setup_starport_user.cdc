// Lock Flow Tokens
import FlowToken from 0x0ae53cb6e3f42a79
import Starport from 0xf8d6e0586b0a20c7

transaction() {

    let participant: Address

    prepare(signer: AuthAccount) {

        self.participant = signer.address

        let starport <- Starport.createStarportParticipant();
		// Store the vault in the account storage
		signer.save<@Starport.StarportParticipant>(<-starport, to: /storage/starportParticipant)

        signer.link<&Starport.StarportParticipant{Starport.FlowLock}>(/public/participant, target: /storage/starportParticipant)

        log("Starport participant was stored")
    }

    execute {
        getAccount(self.participant)
            .getCapability(/public/participant).borrow<&Starport.StarportParticipant{Starport.FlowLock}>() 
            ?? panic("Could not borrow Starport participant")
    }
}