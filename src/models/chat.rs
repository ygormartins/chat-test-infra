/*---------- Imports ----------*/
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum MessageStatus {
    Ok,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ChatType {
    Private,
    Group,
}

impl std::fmt::Display for ChatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_version = match self {
            ChatType::Private => "private",
            ChatType::Group => "group",
        };

        write!(f, "{}", string_version)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
    Text,
    Image,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Text
    }
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string_version = match self {
            MessageType::Text => "text",
            MessageType::Image => "image",
        };

        write!(f, "{}", string_version)
    }
}

fn default_message_content() -> String {
    "".to_owned()
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MessagePayload {
    pub temp_id: String,

    #[serde(default = "default_message_content")]
    pub content: String,

    pub image_url: Option<String>,

    #[serde(default)]
    pub message_type: MessageType,

    pub chat_type: ChatType,

    pub user_sub: Option<String>,

    pub group_id: Option<String>,
}
