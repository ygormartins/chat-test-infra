/*---------- Imports ----------*/
use base64::{engine::general_purpose, Engine};
use serde::de::DeserializeOwned;

pub struct Jwt;

impl Jwt {
    pub fn decode_payload<T: DeserializeOwned>(token: &str) -> Result<T, String> {
        let token_sections: Vec<&str> = token.split(".").collect();
        let payload_section = token_sections[1];

        let decoding_result = general_purpose::STANDARD_NO_PAD.decode(payload_section);

        if let Ok(decoded) = decoding_result {
            if let Ok(result) = serde_json::from_slice::<T>(&decoded) {
                return Ok(result);
            }
        }

        Err("Couldn't decode the token".to_owned())
    }
}
