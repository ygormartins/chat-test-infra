/*---------- Imports ----------*/
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::models::user::User;
use chat_test_infra::utils::jwt::Jwt;
use lambda_http::{service_fn, Error, IntoResponse, Request, RequestExt, Response};
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

    let chat_user_sub = chat_sort_key.replace("chat@user#", "");

    let mut sorted_subs_list = [chat_user_sub, user.sub];
    sorted_subs_list.sort_by(|a, b| b.cmp(a));

    let partition_key = format!("users#{}|{}", sorted_subs_list[0], sorted_subs_list[1]);

    let query_request = dynamodb_client
        .query()
        .table_name(table_name)
        .scan_index_forward(false)
        .expression_attribute_values(":partitionKey", AttributeValue::S(partition_key))
        .expression_attribute_values(":sortKey_prefix", AttributeValue::S("message#".to_owned()))
        .key_condition_expression(
            "partitionKey = :partitionKey and begins_with(sortKey, :sortKey_prefix)",
        )
        .send()
        .await;

    match query_request {
        Ok(query_result) => {
            let items_list = query_result.items().unwrap_or(&[]);
            let unmarshed_items = from_items::<Value>(items_list.to_vec()).unwrap_or(vec![]);

            Ok(Response::builder()
                .status(200)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "*")
                .body(json!({ "data": unmarshed_items }).to_string())?)
        }

        Err(_) => Ok(Response::builder()
            .status(500)
            .header("Access-Control-Allow-Headers", "Content-Type")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .body(
                json!({ "message": "An error happened while querying the resource" }).to_string(),
            )?),
    }
}
