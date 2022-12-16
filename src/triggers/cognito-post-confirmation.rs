/*---------- Imports ----------*/
use aws_lambda_events::cognito::CognitoEventUserPoolsPostConfirmation;
use aws_sdk_s3::types::ByteStream;
use image::ImageOutputFormat;
use initials_revamped::{AvatarBuilder, AvatarResult};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::{env, io::Cursor};

/*---------- Constants ----------*/
const IMAGE_SIZE: u32 = 512;
const FONT_COLOR: &str = "#ffffff";
const FONT_SCALE: f32 = 250.0;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = aws_config::load_from_env().await;
    let public_bucket_name = env::var("PUBLIC_BUCKET").expect("PUBLIC_BUCKET must be set");
    let s3_client = aws_sdk_s3::Client::new(&config);
    let handler = service_fn(|event| handler_fn(&s3_client, &public_bucket_name, event));

    lambda_runtime::run(handler).await?;

    Ok(())
}

fn avatar(user_name: &str) -> AvatarResult {
    AvatarBuilder::new(user_name)
        .with_font_scale(FONT_SCALE)?
        .with_font_color(FONT_COLOR)?
        .with_width(IMAGE_SIZE)?
        .with_height(IMAGE_SIZE)
}

async fn handler_fn(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &str,
    event: LambdaEvent<CognitoEventUserPoolsPostConfirmation>,
) -> Result<CognitoEventUserPoolsPostConfirmation, Error> {
    let user_sub = event.payload.request.user_attributes.get("sub");
    let user_name = event.payload.request.user_attributes.get("name");

    if let (Some(sub_value), Some(name_value)) = (user_sub, user_name) {
        let avatar = avatar(name_value);

        if let Ok(avatar_result) = avatar {
            let image_key = format!("user/{}.png", sub_value);
            let avatar_image = avatar_result.draw();

            let mut image_bytes: Vec<u8> = Vec::new();
            let mut image_cursor = Cursor::new(&mut image_bytes);

            avatar_image.write_to(&mut image_cursor, ImageOutputFormat::Png)?;

            s3_client
                .put_object()
                .bucket(bucket_name)
                .key(image_key)
                .content_type("image/png")
                .body(ByteStream::from(image_bytes))
                .send()
                .await?;
        }
    }

    Ok(event.payload)
}
