// Unlock Flow tokens with given authorities signatures
import Starport from 0xf8d6e0586b0a20c7

transaction(noticeEraId: UInt256,
            noticeEraIndex: UInt256,
            parentNoticeHex: String,
            supplyCap: UFix64,
            signatures: [String]) {

    prepare(signer: AuthAccount) {

    }

    execute {

        Starport.setSupplyCap(
            noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            parentNoticeHex: parentNoticeHex,
            supplyCap: supplyCap,
            signatures: signatures,
        )
    }
}