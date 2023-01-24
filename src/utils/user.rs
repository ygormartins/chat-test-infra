/*---------- Imports ----------*/
use crate::models::user::User as UserModel;
use aws_sdk_cognitoidentityprovider::model::AttributeType;
use serde::de::value::{Error, MapDeserializer};
use serde::Deserialize;
use std::collections::HashMap;

pub struct User;

/*---------- Enums ----------*/
pub enum GetUserError {
    NotFound,
    RequestFailed,
    MissingAttributes,
    InvalidUserSchema,
}

fn generate_attrs_map(attributes_list: &[AttributeType]) -> HashMap<String, String> {
    let mut response: HashMap<String, String> = HashMap::new();

    for attribute in attributes_list.iter() {
        if attribute.name() == Some("name") {
            response.insert(
                "name".to_owned(),
                attribute.value().unwrap_or("").to_owned(),
            );
        };

        if attribute.name() == Some("email") {
            response.insert(
                "email".to_owned(),
                attribute.value().unwrap_or("").to_owned(),
            );
        };

        if attribute.name() == Some("sub") {
            response.insert("sub".to_owned(), attribute.value().unwrap_or("").to_owned());
        };
    }

    response
}

impl User {
    pub async fn get_user_by_sub(
        cognito_client: &aws_sdk_cognitoidentityprovider::Client,
        userpool_id: &str,
        sub: &str,
    ) -> Result<UserModel, GetUserError> {
        let filter_query = format!("sub = \"{}\"", sub);
        let list_users_request = cognito_client
            .list_users()
            .user_pool_id(userpool_id)
            .filter(filter_query)
            .limit(1)
            .send()
            .await;

        let users_list = match list_users_request {
            Ok(list) => list,
            Err(_) => return Err(GetUserError::RequestFailed),
        };

        let user_info = match users_list.users().unwrap_or_default().get(0) {
            Some(info) => info,
            None => return Err(GetUserError::NotFound),
        };

        let user_attributes = match user_info.attributes() {
            Some(attributes) => attributes,
            None => return Err(GetUserError::MissingAttributes),
        };

        let response = generate_attrs_map(user_attributes);

        match UserModel::deserialize(MapDeserializer::<_, Error>::new(response.into_iter())) {
            Ok(parsed) => Ok(parsed),
            Err(_) => Err(GetUserError::InvalidUserSchema),
        }
    }

    pub async fn get_user_by_email(
        cognito_client: &aws_sdk_cognitoidentityprovider::Client,
        userpool_id: &str,
        email: &str,
    ) -> Result<UserModel, GetUserError> {
        let get_user_request = cognito_client
            .admin_get_user()
            .user_pool_id(userpool_id)
            .username(email)
            .send()
            .await;

        let user_info = match get_user_request {
            Ok(info) => info,
            Err(_) => return Err(GetUserError::NotFound),
        };

        let user_attributes = match user_info.user_attributes() {
            Some(attributes) => attributes,
            None => return Err(GetUserError::MissingAttributes),
        };

        let response = generate_attrs_map(user_attributes);

        match UserModel::deserialize(MapDeserializer::<_, Error>::new(response.into_iter())) {
            Ok(parsed) => Ok(parsed),
            Err(_) => Err(GetUserError::InvalidUserSchema),
        }
    }
}
