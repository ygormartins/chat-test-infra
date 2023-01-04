use aws_sdk_cognitoidentityprovider::model::AttributeType;
use lambda_http::{service_fn, Error, IntoResponse, Request, RequestExt, Response};
use serde_json::json;
use std::{env, collections::HashMap};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let userpool_id = env::var("USERPOOL_ID").expect("USERPOOL_ID must be set");
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(&config);
    let handler = service_fn(|request| handler_fn(&cognito_client, &userpool_id, request));

    lambda_http::run(handler).await?;

    Ok(())
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
            response
                .insert("sub".to_owned(), attribute.value().unwrap_or("").to_owned());
        };
    }
    
    response
}

async fn handler_fn(
    cognito_client: &aws_sdk_cognitoidentityprovider::Client,
    userpool_id: &str,
    request: Request,
) -> Result<impl IntoResponse, Error> {
    let query_params = request.query_string_parameters();
    let email_param = query_params.first("email");
    let sub_param = query_params.first("sub");

    match (email_param, sub_param) {
        (Some(email_value),_) => {
            let get_user_request = cognito_client.admin_get_user().user_pool_id(userpool_id).username(email_value).send().await;

            if let Ok(get_user_response) = get_user_request {
                if let Some(user_attributes) = get_user_response.user_attributes() {
                    let response = generate_attrs_map(user_attributes);

                    return Ok(Response::builder()
                        .status(200)
                        .header("Access-Control-Allow-Headers", "Content-Type")
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Access-Control-Allow-Methods", "*")
                        .body(json!(response).to_string())?)
                }
            }
                
        },

        (None,Some(sub_value)) => {
            let filter_query = format!("sub = \"{}\"", sub_value);
            let list_users_request = cognito_client.list_users().user_pool_id(userpool_id).filter(filter_query).limit(1).send().await;

            if let Ok(users_list) = list_users_request {
                if let Some(user_info) = users_list.users().ok_or("User not found")?.get(0) {
                    if let Some(user_attributes) = user_info.attributes() {
                        let response = generate_attrs_map(user_attributes);

                        return Ok(Response::builder()
                            .status(200)
                            .header("Access-Control-Allow-Headers", "Content-Type")
                            .header("Access-Control-Allow-Origin", "*")
                            .header("Access-Control-Allow-Methods", "*")
                            .body(json!(response).to_string())?)
                    }
                }
            }
        },
        
        (None, None) => {
            return Ok(Response::builder()
            .status(400)
            .header("Access-Control-Allow-Headers", "Content-Type")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .body(json!({"message": "You must specify either an email or sub value in the query parameters"}).to_string())?)
        }
    };

    Ok(Response::builder()
        .status(404)
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "*")
        .body(json!({"message": "User not found"}).to_string())?)
}
