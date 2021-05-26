// This script reads the amount of locked Flow tokens

import Starport from 0xf8d6e0586b0a20c7

pub fun main(): UFix64 {

    let supplyCap = Starport.getFlowSupplyCap()

    return supplyCap
}