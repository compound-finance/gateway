// This script returns the message to change authorities of Starport

import Starport from 0xf8d6e0586b0a20c7

pub fun main(noticeEraId: UInt256, noticeEraIndex: UInt256, parentNoticeHex: String, toAddress: Address, amount: UFix64): [UInt8] {

    let message = Starport.buildUnlockMessage(noticeEraId: noticeEraId, noticeEraIndex: noticeEraIndex, parentNoticeHex: parentNoticeHex, toAddress: toAddress, amount: amount)

    return message
}