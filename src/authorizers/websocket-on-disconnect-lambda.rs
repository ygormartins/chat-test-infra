/*---------- Imports ----------*/
use aws_lambda_events::apigw::{ApiGatewayProxyResponse, ApiGatewayWebsocketProxyRequest};
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::{
    models::user::User,
    utils::{http::HttpResponse, jwt::Jwt},
};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::env;

/*---------- Structs ----------*/
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct AuthorizerPayload {
    principal_id: String,
}

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
    if let Some(unparsed_auth_payload) = event.payload.request_context.authorizer {
        let auth_payload: AuthorizerPayload = serde_json::from_value(unparsed_auth_payload)?;
        let user_info = Jwt::decode_payload::<User>(&auth_payload.principal_id)?;
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
