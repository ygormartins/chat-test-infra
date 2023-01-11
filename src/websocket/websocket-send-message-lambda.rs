/*---------- Imports ----------*/
use aws_lambda_events::apigw::{ApiGatewayProxyResponse, ApiGatewayWebsocketProxyRequest};
use aws_sdk_apigatewaymanagement::{config::Builder, types::Blob, Endpoint};
use chat_test_infra::{models::chat::MessageStatus, utils::http::HttpResponse};
use chrono::{SecondsFormat, Utc};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use ulid::Ulid;

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
    let websocket_mgmt_api =
        env::var("WEBSOCKET_MGMT_API").expect("WEBSOCKET_MGMT_API must be set");

    // Initializing APIGateway MGTM client
    let apigtw_client_endpoint = Endpoint::immutable(
        websocket_mgmt_api
            .parse()
            .expect("Failed to parse WebSocket endpoint"),
    );

    let apigtw_client_config = Builder::from(&config)
        .endpoint_resolver(apigtw_client_endpoint)
        .build();

    let apigtw_client = aws_sdk_apigatewaymanagement::Client::from_conf(apigtw_client_config);

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);

    let handler =
        service_fn(|event| handler_fn(&dynamodb_client, &apigtw_client, &table_name, event));

    lambda_runtime::run(handler).await?;

    Ok(())
}

async fn handler_fn(
    _dynamodb_client: &aws_sdk_dynamodb::Client,
    apigtw_client: &aws_sdk_apigatewaymanagement::Client,
    _table_name: &str,
    event: LambdaEvent<ApiGatewayWebsocketProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let message_id = Ulid::new().to_string();
    let message_status = MessageStatus::Ok;
    let current_timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let success_payload = json!({
        "action": "message-status",
        "data": {
            "status": message_status,
            "timestamp": current_timestamp,
            "tempId": "uuid",
            "messageId": message_id,
        }
    });

    apigtw_client
        .post_to_connection()
        .set_connection_id(event.payload.request_context.connection_id)
        .set_data(Some(Blob::new(success_payload.to_string())))
        .send()
        .await?;

    Ok(HttpResponse::build_success_response())
}
