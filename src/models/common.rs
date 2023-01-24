/*---------- Imports ----------*/
use aws_lambda_events::dynamodb::attributes::AttributeValue as EventAttributeValue;
use aws_sdk_dynamodb::model::AttributeValue as DynamoAttributeValue;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketEvent<T> {
    pub action: String,
    pub data: T,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseItem {
    pub partition_key: String,
    pub sort_key: String,
    pub entity_type: String,

    #[serde(rename = "gsi1PK")]
    pub gsi1_pk: Option<String>,

    #[serde(rename = "gsi1SK")]
    pub gsi1_sk: Option<String>,

    #[serde(rename = "gsi2PK")]
    pub gsi2_pk: Option<String>,

    #[serde(rename = "gsi2SK")]
    pub gsi2_sk: Option<String>,
}

fn convert_attribute(attribute: &EventAttributeValue) -> DynamoAttributeValue {
    match attribute {
        EventAttributeValue::String(str_val) => DynamoAttributeValue::S(str_val.to_owned()),

        EventAttributeValue::Number(num_val) => DynamoAttributeValue::N(num_val.to_string()),

        EventAttributeValue::Boolean(bool_value) => {
            DynamoAttributeValue::Bool(bool_value.to_owned())
        }

        EventAttributeValue::Binary(bin_val) => {
            DynamoAttributeValue::B(aws_sdk_dynamodb::types::Blob::new(bin_val.to_owned()))
        }

        EventAttributeValue::StringSet(ss_val) => DynamoAttributeValue::Ss(ss_val.to_owned()),

        EventAttributeValue::NumberSet(ns_val) => {
            let converted_list = ns_val
                .into_iter()
                .map(|current| current.to_string())
                .collect();

            DynamoAttributeValue::Ns(converted_list)
        }

        EventAttributeValue::BinarySet(bs_val) => {
            let converted_list = bs_val
                .into_iter()
                .map(|current| aws_sdk_dynamodb::types::Blob::new(current.to_owned()))
                .collect();

            DynamoAttributeValue::Bs(converted_list)
        }

        EventAttributeValue::AttributeMap(am_val) => {
            let converted_map = parse_attribute_map(am_val);

            DynamoAttributeValue::M(converted_map)
        }

        EventAttributeValue::AttributeList(al_val) => {
            let converted_list = al_val.iter().map(|attr| convert_attribute(attr)).collect();

            DynamoAttributeValue::L(converted_list)
        }

        _ => DynamoAttributeValue::Null(true),
    }
}

fn parse_attribute_map(
    map: &HashMap<String, EventAttributeValue>,
) -> HashMap<String, DynamoAttributeValue> {
    let mut converted_hashmap: HashMap<String, DynamoAttributeValue> = HashMap::new();
    map.into_iter().for_each(|(key, value)| {
        let parsed_value = convert_attribute(value);

        converted_hashmap.insert(key.to_owned(), parsed_value);
    });

    converted_hashmap
}

pub fn parse_event_item<T: DeserializeOwned>(
    item: &HashMap<String, EventAttributeValue>,
) -> Option<T> {
    let parsed_hashmap = parse_attribute_map(item);

    let parsed_message: T = match serde_dynamo::from_item(parsed_hashmap) {
        Ok(parsed) => parsed,
        Err(_) => return None,
    };

    Some(parsed_message)
}
