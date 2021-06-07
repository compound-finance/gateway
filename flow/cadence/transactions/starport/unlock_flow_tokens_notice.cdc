// Unlock Flow tokens with given authorities signatures
import Starport from 0xc8873a26b148ed14

transaction(noticeEraId: UInt256,
            noticeEraIndex: UInt256,
            parentNoticeHex: String,
            signatures: [String],
            toAddress: Address,
            amount: UFix64) {

    prepare(signer: AuthAccount) {
    }

    execute {

        Starport.unlock(
            noticeEraId: noticeEraId,
            noticeEraIndex: noticeEraIndex,
            parentNoticeHex: parentNoticeHex,
            toAddress: toAddress,
            amount: amount,
            signatures: signatures,
        )
    }
}