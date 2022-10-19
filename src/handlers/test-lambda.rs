use lambda_http::{service_fn, Error, IntoResponse, Request, Response};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error> {
    lambda_http::run(service_fn(|request: Request| handler_fn(request))).await?;

    Ok(())
}

async fn handler_fn(request: Request) -> Result<impl IntoResponse, Error> {
    println!("Got called!");
    println!("Event: {:?}", request);

    Ok(Response::builder()
        .status(200)
        .body(json!({"message": "success"}).to_string())?)
}
