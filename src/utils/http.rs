/*---------- Imports ----------*/
use aws_lambda_events::apigw::ApiGatewayProxyResponse;
use serde_json::json;

pub struct HttpResponse;

impl HttpResponse {
    pub fn build_success_response() -> ApiGatewayProxyResponse {
        let response = json!({
            "statusCode": 200,
            "headers": {
                "Access-Control-Allow-Headers": "Content-Type",
                "Access-Control-Allow-Origin": "*",
                "Access-Control-Allow-Methods": "*"
            },
            "multiValueHeaders": {
                "Access-Control-Allow-Headers": ["Content-Type"],
                "Access-Control-Allow-Origin": ["*"],
                "Access-Control-Allow-Methods": ["*"]
            }
        });

        let parsed_reponse: ApiGatewayProxyResponse =
            serde_json::from_value(response).unwrap_or_default();

        parsed_reponse
    }
}
