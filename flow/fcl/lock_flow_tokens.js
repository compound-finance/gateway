import * as fcl from "@onflow/fcl"
import * as t from "@onflow/types"

const txId = await fcl
    .send([
      // Transactions use fcl.transaction instead of fcl.script
      // Their syntax is a little different too
      fcl.transaction`
      import FungibleToken from 0x9a0766d93b6608b7
      import FlowToken from 0x7e60df042a9c0868
      import Starport from 0xc8873a26b148ed14

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
      `,
      fcl.payer(fcl.authz), // current user is responsible for paying for the transaction
      fcl.proposer(fcl.authz), // current user acting as the nonce
      fcl.authorizations([fcl.authz]), // current user will be first AuthAccount
      fcl.limit(70), // set the compute limit
      fcl.args([
        fcl.arg("100.0", t.UFix64), // name
      ])
    ])
    .then(fcl.decode)