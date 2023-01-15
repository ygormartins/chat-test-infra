#![feature(future_join)]

/*---------- Imports ----------*/
use aws_lambda_events::apigw::{ApiGatewayProxyResponse, ApiGatewayWebsocketProxyRequest};
use aws_sdk_apigatewaymanagement::{
    config::Builder,
    error::PostToConnectionError,
    output::PostToConnectionOutput,
    types::{Blob, SdkError},
    Endpoint,
};
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::{
    models::{
        chat::{ChatType, MessagePayload, MessageStatus, MessageType},
        common::WebSocketEvent,
        user::User,
    },
    utils::{http::HttpResponse, jwt::Jwt},
};
use chrono::{SecondsFormat, Utc};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use serde_dynamo::Item;
use serde_json::{json, Value};
use std::{env, future, str::FromStr};
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

    // Initializing APIGateway MGMT client
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

async fn send_websocket_message(
    client: &aws_sdk_apigatewaymanagement::Client,
    connection_id: String,
    message: Value,
) -> Result<PostToConnectionOutput, SdkError<PostToConnectionError>> {
    let send_result = client
        .post_to_connection()
        .set_connection_id(Some(connection_id))
        .set_data(Some(Blob::new(message.to_string())))
        .send()
        .await;

    send_result
}

fn generate_error_message(message: &str) -> Value {
    let status = MessageStatus::Error;
    let message = json!({
        "action": "message-status",
        "data": {
            "status": status,
            "message": message
        }
    });

    message
}

async fn save_private_message(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    table_name: &str,
    message_payload: &MessagePayload,
    message_id: &str,
    message_timestamp: &str,
    user_info: &User,
) -> Result<(), String> {
    let receiver_sub = match &message_payload.user_sub {
        Some(sub) => sub,
        None => "",
    };

    let mut sorted_subs_list = [receiver_sub, &user_info.sub];

    sorted_subs_list.sort_by(|a, b| b.cmp(a));

    let partition_key = format!("users#{}|{}", sorted_subs_list[0], sorted_subs_list[1]);
    let sort_key = format!("message#{}", message_id);

    let user_data_item: Item = match serde_dynamo::to_item(user_info.clone()) {
        Ok(item) => item,
        Err(_) => return Err("Couldn't parse user information".to_string()),
    };

    let put_item_result = dynamodb_client
        .put_item()
        .table_name(table_name)
        .item("partitionKey", AttributeValue::S(partition_key))
        .item("sortKey", AttributeValue::S(sort_key))
        .item("timestamp", AttributeValue::S(message_timestamp.to_owned()))
        .item(
            "messageType",
            AttributeValue::S(message_payload.message_type.to_string()),
        )
        .item("user", AttributeValue::M(user_data_item.into()))
        .item(
            "content",
            AttributeValue::S(message_payload.content.to_owned()),
        )
        .send()
        .await;

    match put_item_result {
        Ok(_) => Ok(()),
        Err(_) => Err("".to_owned()),
    }
}

async fn handle_send_private_message(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    apigtw_client: &aws_sdk_apigatewaymanagement::Client,
    table_name: &str,
    message_payload: &MessagePayload,
    message_id: &str,
    message_timestamp: &str,
    user_info: &User,
) {
    let receiver_sub = match &message_payload.user_sub {
        Some(sub) => sub,
        None => return,
    };

    let partition_key = format!("user#{}", receiver_sub);

    let get_user_connection_result = dynamodb_client
        .get_item()
        .table_name(table_name)
        .key("partitionKey", AttributeValue::S(partition_key.to_owned()))
        .key("sortKey", AttributeValue::S("connection".to_owned()))
        .send()
        .await;

    if let Ok(get_item_output) = get_user_connection_result {
        if let Some(item_data) = get_item_output.item() {
            let connection_id = match item_data.get("connectionId") {
                Some(id) => {
                    let fallback_str = String::from("");
                    id.as_s().unwrap_or(&fallback_str).to_owned()
                }
                None => return,
            };

            let message_type = MessageType::Text;
            let chat_type = ChatType::Private;
            let message_content = &message_payload.content;

            let message_payload = json!({
                "action": "receive-message",
                "data": {
                    "timestamp": message_timestamp,
                    "messageType": message_type,
                    "chatType": chat_type,
                    "content": message_content,
                    "messageId": message_id,
                    "sender": user_info
                }
            });

            send_websocket_message(apigtw_client, connection_id, message_payload)
                .await
                .ok();
        }
    }
}

async fn handler_fn(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    apigtw_client: &aws_sdk_apigatewaymanagement::Client,
    table_name: &str,
    event: LambdaEvent<ApiGatewayWebsocketProxyRequest>,
) -> Result<ApiGatewayProxyResponse, Error> {
    let connection_id = match &event.payload.request_context.connection_id {
        Some(id) => id.to_owned(),
        None => return Ok(HttpResponse::build_success_response()),
    };

    let user_info = match Jwt::get_user_from_payload(&event.payload) {
        Some(user) => user,
        None => {
            let error_message = generate_error_message("Invalid token received");

            send_websocket_message(apigtw_client, connection_id, error_message).await?;

            return Ok(HttpResponse::build_success_response());
        }
    };

    let parsed_body = match Value::from_str(
        &event
            .payload
            .body
            .unwrap_or("No body in request".to_owned()),
    ) {
        Ok(parsed_body) => parsed_body,
        Err(_) => {
            let error_message = generate_error_message("Couldn't parse request body");

            send_websocket_message(apigtw_client, connection_id, error_message).await?;

            return Ok(HttpResponse::build_success_response());
        }
    };

    let message_payload =
        match serde_json::from_value::<WebSocketEvent<MessagePayload>>(parsed_body) {
            Ok(parsed_body) => parsed_body.data,
            Err(_) => {
                let error_message = generate_error_message("Request body failed validation");

                send_websocket_message(apigtw_client, connection_id, error_message).await?;

                return Ok(HttpResponse::build_success_response());
            }
        };

    if let Some(receiver_sub) = &message_payload.user_sub {
        if receiver_sub == &user_info.sub {
            let error_message = generate_error_message("You can't send a message to yourself");

            send_websocket_message(apigtw_client, connection_id, error_message).await?;

            return Ok(HttpResponse::build_success_response());
        }
    }

    let message_id = Ulid::new().to_string();
    let message_status = MessageStatus::Ok;
    let current_timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    let send_msg_result = match message_payload.chat_type {
        _ => {
            let (_, save_result) = future::join!(
                handle_send_private_message(
                    dynamodb_client,
                    apigtw_client,
                    table_name,
                    &message_payload,
                    &message_id,
                    &current_timestamp,
                    &user_info,
                ),
                save_private_message(
                    dynamodb_client,
                    table_name,
                    &message_payload,
                    &message_id,
                    &current_timestamp,
                    &user_info
                )
            )
            .await;

            save_result as Result<(), String>
        }
    };

    let result_payload = match send_msg_result {
        Ok(()) => {
            let success_payload = json!({
                "action": "message-status",
                "data": {
                    "status": message_status,
                    "timestamp": current_timestamp,
                    "tempId": message_payload.temp_id,
                    "messageId": message_id,
                }
            });

            success_payload
        }

        Err(error) => {
            let message = generate_error_message(&error);

            message
        }
    };

    send_websocket_message(apigtw_client, connection_id, result_payload).await?;

    Ok(HttpResponse::build_success_response())
}
