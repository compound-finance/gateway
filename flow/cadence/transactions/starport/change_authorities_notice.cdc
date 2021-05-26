// Unlock Flow tokens with given authorities signatures
import Starport from 0xf8d6e0586b0a20c7

transaction(noticeEraId: UInt256,
            noticeEraIndex: UInt256,
            parentNoticeHex: String,
            newAuthorities: [String],
            signatures: [String]) {

    prepare(signer: AuthAccount) {

    }

    execute {

        Starport.changeAuthorities(
            noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            parentNoticeHex: parentNoticeHex,
            newAuthorities: newAuthorities,
            signatures: signatures,
        )
    }
}