use lambda_http::{service_fn, Error, IntoResponse, Request, RequestExt, Response};
use serde_json::json;
use std::{collections::HashMap, env};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let userpool_id = env::var("USERPOOL_ID").expect("USERPOOL_ID must be set");
    let cognito_client = aws_sdk_cognitoidentityprovider::Client::new(&config);
    let handler = service_fn(|request| handler_fn(&cognito_client, &userpool_id, request));

    lambda_http::run(handler).await?;

    Ok(())
}

async fn handler_fn(
    cognito_client: &aws_sdk_cognitoidentityprovider::Client,
    userpool_id: &str,
    request: Request,
) -> Result<impl IntoResponse, Error> {
    let queryparams = request.query_string_parameters();

    let email_param = match queryparams.first("email") {
        Some(value) => value,
        None => {
            return Ok(Response::builder()
                .status(400)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET")
                .body(json!({"message": "Missing email from query parameters"}).to_string())?)
        }
    };

    let user_details_response = cognito_client
        .admin_get_user()
        .user_pool_id(userpool_id)
        .username(email_param)
        .send()
        .await;

    match user_details_response {
        Ok(details) => {
            let mut response: HashMap<String, String> = HashMap::new();

            if let Some(user_attributes) = details.user_attributes() {
                for attribute in user_attributes.iter() {
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
            }

            Ok(Response::builder()
                .status(200)
                .header("Access-Control-Allow-Headers", "Content-Type")
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET")
                .body(json!(response).to_string())?)
        }

        Err(_) => Ok(Response::builder()
            .status(404)
            .header("Access-Control-Allow-Headers", "Content-Type")
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "GET")
            .body(json!({"message": "User not found"}).to_string())?),
    }
}
