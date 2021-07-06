// This script reads the amount of locked Flow tokens

import Starport from 0xc8873a26b148ed14

pub fun main(): UFix64 {

    let supplyCap = Starport.getFlowSupplyCap()

    return supplyCap
}