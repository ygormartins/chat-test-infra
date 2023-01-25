/*---------- Imports ----------*/
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::models::user::User;
use chat_test_infra::utils::jwt::Jwt;
use lambda_http::{service_fn, Error, IntoResponse, Request, Response};
use serde_dynamo::aws_sdk_dynamodb_0_21::from_items;
use serde_json::{json, Value};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let handler = service_fn(|request| handler_fn(&dynamodb_client, &table_name, request));

    lambda_http::run(handler).await?;

    Ok(())
}

async fn handler_fn(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    request: Request,
) -> Result<impl IntoResponse, Error> {
    let headers = request.headers();

    let id_token = match headers.get("authorization") {
        Some(token) => token.to_str()?,
        None => {
            return Ok(Response::builder()
                .status(400)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "*")
                .body(json!({"message": "Missing authentication token"}).to_string())?)
        }
    };

    let user: User = match Jwt::decode_payload(id_token) {
        Ok(user_obj) => user_obj,
        Err(_) => {
            return Ok(Response::builder()
                .status(400)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "*")
                .body(json!({"message": "Invalid user token"}).to_string())?)
        }
    };

    let query_request = dynamodb_client
        .query()
        .table_name(table_name)
        .index_name("GSI2")
        .expression_attribute_values(":gsi2PK", AttributeValue::S(format!("user#{}", user.sub)))
        .expression_attribute_values(
            ":gsi2SK_prefix",
            AttributeValue::S("chat-timestamp#".to_owned()),
        )
        .key_condition_expression("gsi2PK = :gsi2PK and begins_with(gsi2SK, :gsi2SK_prefix)")
        .send()
        .await;

    if let Ok(query_result) = query_request {
        let items_list = query_result.items().unwrap_or(&[]);
        let unmarshed_items = from_items::<Value>(items_list.to_vec()).unwrap_or(vec![]);

        return Ok(Response::builder()
            .status(200)
            .header("Access-Control-Allow-Headers", "Content-Type")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .body(json!({ "data": unmarshed_items }).to_string())?);
    }

    Ok(Response::builder()
        .status(500)
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .body(json!({"message": "An error ocurred while fetching the chats"}).to_string())?)
}
