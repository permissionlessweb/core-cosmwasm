use crate::msg::{CollectionExtensionMsg, CollectionParams};
use cosmwasm_std::{Decimal, Timestamp};
use cw721::msg::RoyaltyInfoResponse;

pub fn mock_collection_params() -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionExtensionMsg {
            // creator: "creator".to_string(),
            description: Some(String::from("Terp Monkeys")),
            image: Some("https://example.com/image.png".to_string()),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time: None,
            explicit_content: Some(false),
            royalty_info: None,
            // royalty_info: Some(RoyaltyInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
        },
    }
}

pub fn mock_collection_params_1(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: "Collection Name".to_string(),
        symbol: "COL".to_string(),
        info: CollectionExtensionMsg {
            // creator: "creator".to_string(),
            description: Some(String::from("Terp Monkeys")),
            image: Some("https://example.com/image.png".to_string()),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time,
            explicit_content: Some(false),
            royalty_info: None,
            // royalty_info: Some(RoyaltyInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
        },
    }
}

pub fn mock_curator_payment_address(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        info: CollectionExtensionMsg {
            // creator: "creator".to_string(),
            description: Some(String::from("Terp Monkeys")),
            image: Some("https://example.com/image.png".to_string()),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time,
            explicit_content: Some(false),
            royalty_info: None,
            // royalty_info: Some(RoyaltyInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
        },
    }
}

pub fn mock_collection_params_high_fee(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Coin"),
        symbol: String::from("TEST"),
        info: CollectionExtensionMsg {
            // creator: "creator".to_string(),
            description: Some(String::from("Terp Monkeys")),
            image: Some("https://example.com/image.png".to_string()),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time,
            explicit_content: Some(false),
            royalty_info: None,
            // royalty_info: Some(RoyaltyInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(100),
            // }),
        },
    }
}

pub fn mock_collection_two(start_trading_time: Option<Timestamp>) -> CollectionParams {
    CollectionParams {
        code_id: 1,
        name: String::from("Test Collection 2"),
        symbol: String::from("TEST 2"),
        info: CollectionExtensionMsg {
            // creator: "creator".to_string(),
            description: Some(String::from("Terp Monkeys 2")),
            image: Some(
                "ipfs://bafybeigi3bwpvyvsmnbj46ra4hyffcxdeaj6ntfk5jpic5mx27x6ih2qvq".to_string(),
            ),
            external_link: Some("https://example.com/external.html".to_string()),
            start_trading_time,
            explicit_content: Some(false),
            royalty_info: None,
            // royalty_info: Some(RoyaltyInfoResponse {
            //     payment_address: "creator".to_string(),
            //     share: Decimal::percent(10),
            // }),
        },
    }
}
