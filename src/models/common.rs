/*---------- Imports ----------*/
use serde::{Deserialize, Serialize};

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
