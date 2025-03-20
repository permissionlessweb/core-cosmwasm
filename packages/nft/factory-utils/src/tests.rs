use crate::msg::CollectionParams;
use cosmwasm_std::Timestamp;
use cw721::msg::CollectionInfoAndExtensionResponse;

pub fn mock_collection_params() -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionInfoAndExtensionResponse {
            // creator: "creator".to_string(),
            // description: String::from("Terp Monkeys"),
            // image: "https://example.com/image.png".to_string(),
            // external_link: Some("https://example.com/external.html".to_string()),
            // start_trading_time,
            // explicit_content: Some(false),
            // residual_info: Some(ResidualInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
            name: todo!(),
            symbol: todo!(),
            extension: todo!(),
            updated_at: todo!(),
        },
    }
}

pub fn mock_collection_params_1(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionInfoAndExtensionResponse {
            // creator: "creator".to_string(),
            // description: String::from("Terp Monkeys"),
            // image: "https://example.com/image.png".to_string(),
            // external_link: Some("https://example.com/external.html".to_string()),
            // start_trading_time,
            // explicit_content: Some(false),
            // residual_info: Some(ResidualInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
            name: todo!(),
            symbol: todo!(),
            extension: todo!(),
            updated_at: todo!(),
        },
    }
}

pub fn mock_curator_payment_address(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        info: CollectionInfoAndExtensionResponse {
            // creator: "creator".to_string(),
            // description: String::from("Terp Monkeys"),
            // image: "https://example.com/image.png".to_string(),
            // external_link: Some("https://example.com/external.html".to_string()),
            // start_trading_time,
            // explicit_content: Some(false),
            // residual_info: Some(ResidualInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
            name: todo!(),
            symbol: todo!(),
            extension: todo!(),
            updated_at: todo!(),
        },
    }
}

pub fn mock_collection_params_high_fee(start_trading_time: Option<Timestamp>) -> CollectionParams {
    // CollectionParams {
    //     code_id: 1,
    //     name: String::from("Test Coin"),
    //     symbol: String::from("TEST"),
    //     info: CollectionInfo {
    //         creator: "creator".to_string(),
    //         description: String::from("Terp Monkeys"),
    //         image:
    //             "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
    //                 .to_string(),
    //         external_link: Some("https://example.com/external.html".to_string()),
    //         residual_info: Some(ResidualInfoResponse {
    //             payment_address: "creator".to_string(),
    //             share: Decimal::percent(100),
    //         }),
    //         start_trading_time,
    //         explicit_content: None,
    //     },
    // }
    CollectionParams {
        code_id: 1,
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        info: CollectionInfoAndExtensionResponse {
            // creator: "creator".to_string(),
            // description: String::from("Terp Monkeys"),
            // image: "https://example.com/image.png".to_string(),
            // external_link: Some("https://example.com/external.html".to_string()),
            // start_trading_time,
            // explicit_content: Some(false),
            // residual_info: Some(ResidualInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
            name: todo!(),
            symbol: todo!(),
            extension: todo!(),
            updated_at: todo!(),
        },
        // info: CollectionInfo {
        //     creator: "creator".to_string(),
        //     description: String::from("Terp Monkeys"),
        //     image:
        //         "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
        //             .to_string(),
        //     external_link: Some("https://example.com/external.html".to_string()),
        //     residual_info: Some(ResidualInfoResponse {
        //         payment_address: "creator".to_string(),
        //         share: Decimal::percent(100),
        //     }),
        //     start_trading_time,
        //     explicit_content: None,
        // },
    }
}

pub fn mock_collection_two(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Collection 2"),
        symbol: String::from("TEST 2"),
        info: CollectionInfoAndExtensionResponse {
            // creator: "creator".to_string(),
            // description: String::from("Terp Monkeys"),
            // image: "https://example.com/image.png".to_string(),
            // external_link: Some("https://example.com/external.html".to_string()),
            // start_trading_time,
            // explicit_content: Some(false),
            // residual_info: Some(ResidualInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
            name: todo!(),
            symbol: todo!(),
            extension: todo!(),
            updated_at: todo!(),
        },
        // info: CollectionInfo {
        //     creator: "creator".to_string(),
        //     description: String::from("Terp Monkeys 2"),
        //     image:
        //         "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq/images/1.png"
        //             .to_string(),
        //     external_link: Some("https://example.com/external.html".to_string()),
        //     residual_info: Some(ResidualInfoResponse {
        //         payment_address: "creator".to_string(),
        //         share: Decimal::percent(10),
        //     }),
        //     start_trading_time,
        //     explicit_content: None,
        // },
    }
}
