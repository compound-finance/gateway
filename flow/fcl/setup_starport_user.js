import * as fcl from "@onflow/fcl"
import * as t from "@onflow/types"

const txId = await fcl
    .send([
      // Transactions use fcl.transaction instead of fcl.script
      // Their syntax is a little different too
      fcl.transaction`
        import FlowToken from 0x7e60df042a9c0868
        import Starport from 0xc8873a26b148ed14
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
      `,
      fcl.payer(fcl.authz), // current user is responsible for paying for the transaction
      fcl.proposer(fcl.authz), // current user acting as the nonce
      fcl.authorizations([fcl.authz]), // current user will be first AuthAccount
      fcl.limit(70), // set the compute limit
    ])
    .then(fcl.decode)