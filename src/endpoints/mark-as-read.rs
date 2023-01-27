/*---------- Imports ----------*/
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::models::user::User;
use chat_test_infra::utils::jwt::Jwt;
use lambda_http::{service_fn, Error, IntoResponse, Request, RequestExt, Response};
use serde_json::json;
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
    let query_parameters = request.query_string_parameters();

    let chat_sort_key = match query_parameters.first("chatSortKey") {
        Some(key) => key,
        None => {
            return Ok(Response::builder()
                .status(400)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "*")
                .body(json!({"message": "Missing sort key in query params"}).to_string())?)
        }
    };

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

    let partition_key = format!("user#{}", user.sub);

    let update_request = dynamodb_client
        .update_item()
        .table_name(table_name)
        .key("partitionKey", AttributeValue::S(partition_key))
        .key("sortKey", AttributeValue::S(chat_sort_key.to_owned()))
        .expression_attribute_values(":unreadMessages", AttributeValue::N(0.to_string()))
        .update_expression("SET unreadMessages = :unreadMessages")
        .condition_expression("attribute_exists(partitionKey) and attribute_exists(sortKey)")
        .send()
        .await;

    let (message, status) = match update_request {
        Ok(_) => (
            String::from("Succesfully updated the requested resource"),
            200,
        ),

        Err(_) => (String::from("Resource not found"), 404),
    };

    Ok(Response::builder()
        .status(status)
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .body(json!({ "message": message }).to_string())?)
}
