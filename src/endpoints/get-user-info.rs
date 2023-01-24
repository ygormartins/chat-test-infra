use chat_test_infra::utils::user::User;
use lambda_http::{service_fn, Error, IntoResponse, Request, RequestExt, Response};
use serde_json::json;
use std::env;

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
    let query_params = request.query_string_parameters();
    let email_param = query_params.first("email");
    let sub_param = query_params.first("sub");

    match (email_param, sub_param) {
        (Some(email_value),_) => {
            let user_info_req = User::get_user_by_email(cognito_client, userpool_id, email_value).await;

            if let Ok(user_info) = user_info_req {
                return Ok(Response::builder()
                    .status(200)
                    .header("Access-Control-Allow-Headers", "Content-Type")
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Methods", "*")
                    .body(json!(user_info).to_string())?)
            }
                
        },

        (None,Some(sub_value)) => {
            let user_info_req = User::get_user_by_sub(cognito_client, userpool_id, sub_value).await;

            if let Ok(user_info) = user_info_req {
                return Ok(Response::builder()
                    .status(200)
                    .header("Access-Control-Allow-Headers", "Content-Type")
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Access-Control-Allow-Methods", "*")
                    .body(json!(user_info).to_string())?)
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
