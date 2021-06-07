// This script reads an era id

import Starport from 0xc8873a26b148ed14

pub fun main(): UInt256 {

    let eraId = Starport.getEraId()

    return eraId
}