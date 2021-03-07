#![feature(proc_macro_hygiene, decl_macro, array_methods)]

#[macro_use]
extern crate rocket;

use chrono::prelude::*;
use gateway_crypto::{
    bytes_to_eth_hex_string, public_key_bytes_to_eth_address, Keyring,
    ETH_KEY_ID_ENV_VAR_DEV_DEFAULT,
};
use rocket::{Rocket, State};
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Sadly this is also defined in /pallets/cash/src/oracle.rs :(
#[derive(Deserialize, Serialize)]
pub struct OpenPriceFeedApiResponse {
    pub messages: Vec<String>,
    pub prices: HashMap<String, String>,
    pub signatures: Vec<String>,
    pub timestamp: String,
}

/// A post body for pushing new prices
#[derive(Deserialize)]
pub struct PostPriceBody {
    pub key: String,
    pub value: u64,
}

/// This holds our state
struct App {
    pub keyring: Mutex<gateway_crypto::InMemoryKeyring>,
    pub key_id: gateway_crypto::KeyId,
    pub prices: Mutex<HashMap<String, u64>>,
}

impl App {
    /// Creates a new mock open oracle reporter with an Eth price of 1000.000001 using the alice key
    /// The reporter address is 0xb4521c6e39dfbad1c654990757c530b9c292ed61
    fn new() -> App {
        let keyring = Mutex::new(gateway_crypto::dev_keyring());
        let key_id: gateway_crypto::KeyId = ETH_KEY_ID_ENV_VAR_DEV_DEFAULT.into();
        let mut prices: HashMap<String, u64> = HashMap::new();
        prices.insert("ETH".into(), 1000000001);
        let prices = Mutex::new(prices);

        App {
            keyring,
            key_id,
            prices,
        }
    }

    /// Get the address of this reporter
    fn get_reporter_eth_address(self: &Self) -> String {
        // obviously not for production..
        let pubkey = self
            .keyring
            .lock()
            .unwrap()
            .get_public_key(&self.key_id)
            .unwrap();
        let addr = public_key_bytes_to_eth_address(&pubkey);
        bytes_to_eth_hex_string(&addr)
    }

    /// Get the typical open api response
    fn get_open_api_response(self: &Self) -> String {
        let keyring = self.keyring.lock().unwrap();
        let prices = self.prices.lock().unwrap();
        let kind = "prices";
        let timestamp = Utc::now().timestamp() as u64;

        let kind = ethabi::Token::String(kind.into());
        let timestamp_eth_abi = ethabi::Token::Uint(timestamp.into());

        let mut message_strings = Vec::new();
        let mut digested_message_bytes_to_sign: Vec<gateway_crypto::HashedMessageBytes> =
            Vec::new();
        let mut price_strings = HashMap::new();
        let mut digested: gateway_crypto::HashedMessageBytes;

        for (key, value) in prices.iter() {
            let value: u64 = *value;
            let ethabi_key = ethabi::Token::String(key.clone());
            let ethabi_value = ethabi::Token::Uint(value.into());
            let ethabi_encoded_bytes = ethabi::encode(&[
                kind.clone(),
                timestamp_eth_abi.clone(),
                ethabi_key,
                ethabi_value,
            ]);
            digested = gateway_crypto::keccak(&ethabi_encoded_bytes);
            let hex_encoded_string = gateway_crypto::bytes_to_eth_hex_string(&ethabi_encoded_bytes);
            message_strings.push(hex_encoded_string);
            digested_message_bytes_to_sign.push(digested);
            // todo: this may cause issues later with prices not matching during sanity check
            price_strings.insert(key.clone(), format!("{}", (value as f64) / (1000000.0)));
        }
        let digested_message_bytes_to_sign: Vec<&[u8]> = digested_message_bytes_to_sign
            .iter()
            .map(|e| e.as_slice())
            .collect();

        let signatures: Vec<String> = keyring
            .sign(digested_message_bytes_to_sign, &self.key_id)
            .unwrap()
            .iter()
            .map(|e| {
                let bytes = e.as_ref().unwrap();
                gateway_crypto::bytes_to_eth_hex_string(&bytes.as_slice())
            })
            .collect();

        let response = OpenPriceFeedApiResponse {
            messages: message_strings,
            prices: price_strings,
            signatures,
            timestamp: format!("{}", timestamp),
        };

        serde_json::to_string_pretty(&response).unwrap()
    }

    /// Set the price for the given symbol, 6 decimals of precision, USD value
    fn set_price(self: &Self, key: String, value: u64) {
        let mut prices = self.prices.lock().unwrap();
        prices.insert(key.clone(), value);
    }

    /// Get the rocket instance
    fn get_rocket(self: Self) -> Rocket {
        rocket::ignite()
            .manage(self)
            .mount("/", routes![root, get_address, post_price])
    }

    /// Run the app
    fn run(self: Self) {
        self.get_rocket().launch();
    }
}

/** routes **/

#[get("/")]
fn root(app: State<App>) -> String {
    app.get_open_api_response()
}

#[get("/address")]
fn get_address(app: State<App>) -> String {
    app.get_reporter_eth_address()
}

#[post("/price", data = "<body>")]
fn post_price(body: Json<PostPriceBody>, app: State<App>) -> String {
    app.set_price(body.key.clone(), body.value);
    "{}".into()
}

fn main() {
    let app = App::new();
    app.run();
}

/** tests **/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_integration_happy_path() {
        let app = App::new();
        let rocket = app.get_rocket();
        let client = rocket::local::Client::untracked(rocket).unwrap();

        // set a price
        let price_body = r#"{"key": "ETH", "value": 2000000000}"#;
        let resp = client.post("/price").body(price_body).dispatch();
        assert_eq!(resp.status(), rocket::http::Status::Ok);

        // read it back
        let mut resp = client.get("/").dispatch();
        assert_eq!(resp.status(), rocket::http::Status::Ok);
        let body = resp.body_string().unwrap();
        let deserialized: OpenPriceFeedApiResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(deserialized.messages.len(), 1);
        assert_eq!(deserialized.signatures.len(), 1);
        assert!(deserialized.timestamp.len() > 0);
        assert!(deserialized.prices.len() == 1);
        assert!(deserialized.prices.contains_key("ETH"));
        assert_eq!(deserialized.prices.get("ETH").unwrap(), "2000")

        // if we really want to test this bad body out we should pull in the logic to check sigs etc
        // from pallets/cash/oracle.rs but this is enough for today
    }
}
