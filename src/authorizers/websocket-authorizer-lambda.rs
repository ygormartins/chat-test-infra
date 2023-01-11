/*---------- Imports ----------*/
use aws_sdk_dynamodb::model::AttributeValue;
use chat_test_infra::models::user::User;
use jsonwebtokens_cognito::KeySet;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env};

/*---------- Structs ----------*/
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2CustomAuthorizerRequestContext {
    api_id: Option<String>,
    route_key: Option<String>,
    request_id: Option<String>,
    connection_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2CustomAuthorizerRequest {
    method_arn: Option<String>,
    cookies: Option<Vec<String>>,
    headers: HashMap<String, String>,
    query_string_parameters: HashMap<String, String>,
    request_context: ApiGatewayV2CustomAuthorizerRequestContext,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct IAMStatement {
    action: String,
    effect: String,
    resource: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
struct PolicyDoument {
    version: String,
    statement: Vec<IAMStatement>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApiGatewayV2CustomAuthorizerResponse {
    principal_id: String,
    policy_document: PolicyDoument,
}

fn generate_policy(principal_id: String, resource: String) -> ApiGatewayV2CustomAuthorizerResponse {
    let api_statement = IAMStatement {
        action: "execute-api:Invoke".to_owned(),
        effect: "Allow".to_owned(),
        resource,
    };

    let policy_document = PolicyDoument {
        version: "2012-10-17".to_owned(),
        statement: vec![api_statement],
    };

    let response = ApiGatewayV2CustomAuthorizerResponse {
        policy_document,
        principal_id,
    };

    response
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let region = config.region().expect("REGION not found").to_string();
    let userpool_id = env::var("USERPOOL_ID").expect("USERPOOL_ID must be set");
    let client_id = env::var("CLIENT_ID").expect("CLIENT_ID must be set");
    let table_name = env::var("TABLE_NAME").expect("TABLE_NAME must be set");
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let handler = service_fn(|event| {
        handler_fn(
            &dynamodb_client,
            &region,
            &userpool_id,
            &client_id,
            &table_name,
            event,
        )
    });

    lambda_runtime::run(handler).await?;

    Ok(())
}

async fn handler_fn(
    dynamodb_client: &aws_sdk_dynamodb::Client,
    region: &str,
    userpool_id: &str,
    client_id: &str,
    table_name: &str,
    event: LambdaEvent<ApiGatewayV2CustomAuthorizerRequest>,
) -> Result<ApiGatewayV2CustomAuthorizerResponse, Error> {
    let id_token_option = event.payload.query_string_parameters.get("idToken");

    if let Some(id_token) = id_token_option {
        let keyset_result = KeySet::new(region, userpool_id);

        if let Ok(keyset) = keyset_result {
            let verifier = keyset.new_id_token_verifier(&[client_id]).build()?;

            let verify_result = keyset.verify(&id_token, &verifier).await;

            match (
                verify_result,
                event.payload.method_arn,
                event.payload.request_context.connection_id,
            ) {
                (Ok(unparsed_user_info), Some(method_arn), Some(connection_id)) => {
                    let user_info: User = serde_json::from_value(unparsed_user_info)?;
                    let partition_key = format!("user#{}", user_info.sub);

                    dynamodb_client
                        .put_item()
                        .table_name(table_name)
                        .item("partitionKey", AttributeValue::S(partition_key.to_owned()))
                        .item("sortKey", AttributeValue::S("connection".to_owned()))
                        .item("gsi1PK", AttributeValue::S("connection".to_owned()))
                        .item("gsi1SK", AttributeValue::S(partition_key.to_owned()))
                        .item("connectionId", AttributeValue::S(connection_id.to_owned()))
                        .item("entityType", AttributeValue::S("connection".to_owned()))
                        .send()
                        .await?;

                    let response = generate_policy(id_token.to_owned(), method_arn.to_owned());

                    return Ok(response);
                }

                _ => {}
            }
        }
    }

    Err("Unauthorized".into())
}
