/*---------- Imports ----------*/
use crate::models::user::User;
use aws_lambda_events::apigw::ApiGatewayWebsocketProxyRequest;
use base64::{engine::general_purpose, Engine};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/*---------- Structs ----------*/
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AuthorizerPayload {
    principal_id: String,
}

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

    pub fn get_user_from_payload(payload: &ApiGatewayWebsocketProxyRequest) -> Option<User> {
        let owned_payload = payload.clone();
        let parse_auth_payload_result =
            serde_json::from_value::<AuthorizerPayload>(owned_payload.request_context.authorizer?);

        let auth_payload = match parse_auth_payload_result {
            Ok(parsed_result) => parsed_result,
            Err(_) => return None,
        };

        Self::decode_payload::<User>(&auth_payload.principal_id).ok()
    }
}
