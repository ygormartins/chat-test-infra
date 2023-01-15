/*---------- Imports ----------*/
use aws_lambda_events::apigw::{ApiGatewayProxyResponse, ApiGatewayWebsocketProxyRequest};
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::utils::{http::HttpResponse, jwt::Jwt};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let handler = service_fn(|event| handler_fn(&dynamodb_client, &table_name, event));

    lambda_runtime::run(handler).await?;

    Ok(())
}

async fn handler_fn(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    event: LambdaEvent<ApiGatewayWebsocketProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let user_info_option = Jwt::get_user_from_payload(&event.payload);

    if let Some(user_info) = user_info_option {
        let partition_key = format!("user#{}", user_info.sub);

        dynamodb_client
            .delete_item()
            .table_name(table_name)
            .key("partitionKey", AttributeValue::S(partition_key))
            .key("sortKey", AttributeValue::S("connection".to_owned()))
            .send()
            .await
            .ok();
    }

    Ok(HttpResponse::build_success_response())
}
