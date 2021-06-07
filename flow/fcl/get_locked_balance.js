import * as fcl from "@onflow/fcl"
import * as t from "@onflow/types"

await fcl.send([
    fcl.script`
        import Starport from 0xc8873a26b148ed14

        pub fun main(): UFix64 {
            let amount = Starport.getLockedBalance()
            return amount
        }
    `
  ]).then(fcl.decode)