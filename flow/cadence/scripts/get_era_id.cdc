// This script reads an era id

import Starport from 0xf8d6e0586b0a20c7

pub fun main(): UInt256 {

    let eraId = Starport.getEraId()

    return eraId
}