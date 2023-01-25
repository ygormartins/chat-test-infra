/*---------- Imports ----------*/
use crate::utils::user::User;
use crate::{
    models::{
        chat::{ChatType, Message},
        common::parse_event_item,
        user::User as UserModel,
    },
    utils::user::GetUserError,
};
use aws_config::SdkConfig;
use aws_lambda_events::dynamodb::EventRecord;
use aws_sdk_dynamodb::model::{AttributeValue, PutRequest, WriteRequest};
use serde_dynamo::aws_sdk_dynamodb_0_21::to_item;
use std::{collections::HashMap, env, future};

async fn create_private_chats(
    dynamo_client: &aws_sdk_dynamodb::Client,
    cognito_client: &aws_sdk_cognitoidentityprovider::Client,
    userpool_id: &str,
    table_name: &str,
    record: &Message,
) -> Result<(), ()> {
    let subs_str = &record.db_item.partition_key.replace("users#", "");
    let subs_list: Vec<&str> = subs_str.split("|").collect();

    let (first_user_info_req, second_user_info_req): (
        Result<UserModel, GetUserError>,
        Result<UserModel, GetUserError>,
    ) = future::join!(
        User::get_user_by_sub(cognito_client, userpool_id, subs_list[0]),
        User::get_user_by_sub(cognito_client, userpool_id, subs_list[1])
    )
    .await;

    let (first_user_info, second_user_info) = match (first_user_info_req, second_user_info_req) {
        (Ok(first_info), Ok(second_info)) => (first_info, second_info),
        _ => return Err(()),
    };

    let (parsed_first_user, parsed_second_user) =
        match (to_item(&first_user_info), to_item(&second_user_info)) {
            (Ok(parsed_first), Ok(parsed_second)) => (parsed_first, parsed_second),
            _ => return Err(()),
        };

    let last_message: HashMap<String, AttributeValue> = HashMap::from([
        (
            "userName".to_owned(),
            AttributeValue::S(record.user.name.to_owned()),
        ),
        (
            "userSub".to_owned(),
            AttributeValue::S(record.user.sub.to_owned()),
        ),
        (
            "timestamp".to_owned(),
            AttributeValue::S(record.timestamp.to_owned()),
        ),
        (
            "preview".to_owned(),
            AttributeValue::S(record.content.to_owned()),
        ),
        (
            "messageType".to_owned(),
            AttributeValue::S(record.message_type.to_string()),
        ),
    ]);

    let base_item: HashMap<String, AttributeValue> = HashMap::from([
        (
            "chatType".to_owned(),
            AttributeValue::S(ChatType::Private.to_string()),
        ),
        (
            "entityType".to_owned(),
            AttributeValue::S("chat".to_owned()),
        ),
        ("lastMessage".to_owned(), AttributeValue::M(last_message)),
        (
            "gsi2SK".to_owned(),
            AttributeValue::S(format!("chat-timestamp#{}", record.timestamp)),
        ),
    ]);

    let (first_unread_msgs, second_unread_msgs) = match record.user.sub == subs_list[0] {
        true => (0, 1),
        false => (1, 0),
    };

    let mut first_item = base_item.clone();
    first_item.insert(
        "partitionKey".to_owned(),
        AttributeValue::S(format!("user#{}", subs_list[0])),
    );
    first_item.insert(
        "sortKey".to_owned(),
        AttributeValue::S(format!("chat@user#{}", subs_list[1])),
    );
    first_item.insert(
        "gsi2PK".to_owned(),
        AttributeValue::S(format!("user#{}", subs_list[0])),
    );
    first_item.insert("user".to_owned(), AttributeValue::M(parsed_first_user));
    first_item.insert("title".to_owned(), AttributeValue::S(second_user_info.name));
    first_item.insert(
        "unreadMessages".to_owned(),
        AttributeValue::N(first_unread_msgs.to_string()),
    );

    let mut second_item = base_item.clone();
    second_item.insert(
        "partitionKey".to_owned(),
        AttributeValue::S(format!("user#{}", subs_list[1])),
    );
    second_item.insert(
        "sortKey".to_owned(),
        AttributeValue::S(format!("chat@user#{}", subs_list[0])),
    );
    second_item.insert(
        "gsi2PK".to_owned(),
        AttributeValue::S(format!("user#{}", subs_list[1])),
    );
    second_item.insert("user".to_owned(), AttributeValue::M(parsed_second_user));
    second_item.insert("title".to_owned(), AttributeValue::S(first_user_info.name));
    second_item.insert(
        "unreadMessages".to_owned(),
        AttributeValue::N(second_unread_msgs.to_string()),
    );

    let operation = dynamo_client
        .batch_write_item()
        .request_items(
            table_name,
            vec![
                WriteRequest::builder()
                    .put_request(PutRequest::builder().set_item(Some(first_item)).build())
                    .build(),
                WriteRequest::builder()
                    .put_request(PutRequest::builder().set_item(Some(second_item)).build())
                    .build(),
            ],
        )
        .send()
        .await;

    match operation {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

pub async fn handler(record: &EventRecord, config: &SdkConfig) {
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let userpool_id = env::var("USERPOOL_ID").expect("USERPOOL_ID must be set");
    let dynamodb_client = aws_sdk_dynamodb::Client::new(config);
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(config);

    let parsed_record = match parse_event_item(&record.change.new_image) {
        Some(parsed) => parsed,
        None => return,
    };

    create_private_chats(
        &dynamodb_client,
        &cognito_client,
        &userpool_id,
        &table_name,
        &parsed_record,
    )
    .await
    .ok();
}
