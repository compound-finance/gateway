// This script returns the message to change authorities of Starport

import Starport from 0xc8873a26b148ed14

pub fun main(noticeEraId: UInt256, noticeEraIndex: UInt256, parentNoticeHex: String, supplyCap: UFix64): [UInt8] {

    let message = Starport.buildSetSupplyCapMessage(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex, parentNoticeHex: parentNoticeHex, supplyCap: supplyCap)

    return message
}