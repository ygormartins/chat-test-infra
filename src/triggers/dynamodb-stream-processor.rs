/*---------- Imports ----------*/
use aws_config::SdkConfig;
use aws_lambda_events::{
    dynamodb::{attributes::AttributeValue, Event, EventRecord},
    event::streams::DynamoDbEventResponse,
};
use chat_test_infra::handlers;
use lambda_runtime::{service_fn, Error, LambdaEvent};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let handler = service_fn(|event| handler_fn(&config, event));

    lambda_runtime::run(handler).await?;

    Ok(())
}

fn get_entity_type(record: &EventRecord, record_event_type: &str) -> Option<String> {
    let record_entity_type_item = match record_event_type {
        "INSERT" | "UPDATE" => {
            let new_image = &record.change.new_image;
            let entity_type_option = new_image.get("entityType");

            match entity_type_option {
                Some(entity_type) => entity_type,
                None => return None,
            }
        }
        "REMOVE" => {
            let old_image = &record.change.old_image;
            let entity_type_option = old_image.get("entityType");

            match entity_type_option {
                Some(entity_type) => entity_type,
                None => return None,
            }
        }
        _ => return None,
    };

    match record_entity_type_item {
        AttributeValue::String(value) => Some(value.to_owned()),
        _ => None,
    }
}

async fn handler_fn(
    config: &SdkConfig,
    event: LambdaEvent<Event>,
) -> Result<DynamoDbEventResponse, Error> {
    let records_iterator = event.payload.records.iter();

    for record in records_iterator {
        let record_event_type = &record.event_name;
        let record_entity_type = match get_entity_type(record, record_event_type) {
            Some(entity_type) => entity_type,
            None => break,
        };

        match (record_event_type.as_str(), record_entity_type.as_str()) {
            ("INSERT", "message") => {
                handlers::message_insert_event::handler(record, config).await;
            }
            _ => break,
        }
    }

    Ok(DynamoDbEventResponse {
        batch_item_failures: vec![],
    })
}
