pub mod json_responses {
    pub const EVENTS_RESPONSE: &[u8] = br#"{
        "jsonrpc":"2.0",
        "id":1,
        "result": [
            {
                "address":"0xbbde1662bc3ed16aa8c618c9833c801f3543b587",
                "blockHash":"0xc1c0eb37b56923ad9e20fdb31ca882988d5217f7ca24b6297ca6ed700811cf23",
                "blockNumber":"0x3adf2f",
                "data":"0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
                "logIndex":"0x0",
                "removed":false,
                "topics":[
                    "0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386",
                    "0x000000000000000000000000eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
                    "0x000000000000000000000000feb1ea27f888c384f1b0dc14fd6b387d5ff47031",
                    "0x513c1ff435eccedd0fda5edd2ad5e5461f0e8726000000000000000000000000"
                ],
                "transactionHash":"0x680e1e81385151f5d791fab0a3c06b03d29b46df08a312d0304cd6a4fc5a7370",
                "transactionIndex":"0x0"
            },
            {
                "address":"0xbbde1662bc3ed16aa8c618c9833c801f3543b587",
                "blockHash":"0xa5c8024e699a5c30eb965e47b5157c06c76f3b726bff377a0a5333a561f25648",
                "blockNumber":"0x3c02e1",
                "data":"0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
                "logIndex":"0x1",
                "removed":false,
                "topics":[
                    "0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386",
                    "0x000000000000000000000000d87ba7a50b2e7e660f678a895e4b72e7cb4ccd9c",
                    "0x000000000000000000000000feb1ea27f888c384f1b0dc14fd6b387d5ff47031",
                    "0xfeb1ea27f888c384f1b0dc14fd6b387d5ff47031000000000000000000000000"
                ],
                "transactionHash":"0x7357859bd05b4429dac758df67f93adb54caad72dd992317811927232c592d4a",
                "transactionIndex":"0x0"
            },
            {
                "address":"0xbbde1662bc3ed16aa8c618c9833c801f3543b587",
                "blockHash":"0xa4a96e957718e3a30b77a667f93978d8f438bdcd56ff03545f08c833d9a26687",
                "blockNumber":"0x3c030b",
                "data":"0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000000000000000000000000000000de0b6b3a764000000000000000000000000000000000000000000000000000000000000000000034554480000000000000000000000000000000000000000000000000000000000",
                "logIndex":"0xe",
                "removed":false,
                "topics":[
                    "0xc459acef3ffe957663bb49d644b20d0c790bcb41573893752a72ba6f023b9386",
                    "0x000000000000000000000000e4e81fa6b16327d4b78cfeb83aade04ba7075165",
                    "0x000000000000000000000000feb1ea27f888c384f1b0dc14fd6b387d5ff47031",
                    "0xfeb1ea27f888c384f1b0dc14fd6b387d5ff47031000000000000000000000000"
                ],
                "transactionHash":"0xad28d82aa1f55e5f965c1da2d84cce29bdb75a134b8f7857c897736c4e562300",
                "transactionIndex":"0x4"
            }
        ]
    }"#;

    pub const NO_EVENTS_RESPONSE: &[u8] = br#"{
        "jsonrpc":"2.0",
        "id":1,
        "result": []
    }"#;

    pub const BLOCK_NUMBER_RESPONSE: &[u8] = br#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": "0xb27467"
    }"#;
}
