/*---------- Imports ----------*/
use aws_lambda_events::cognito::CognitoEventUserPoolsPostConfirmation;
use aws_sdk_s3::types::ByteStream;
use image::{ImageBuffer, ImageOutputFormat, Rgba};
use initials::{AvatarBuilder, AvatarResult};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use std::{env, io::Cursor};

/*---------- Constants ----------*/
const IMAGE_SIZE: u32 = 512;

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
        .with_font_scale(250.0)?
        .with_font_color("#ffffff")?
        .with_font("/assets/fonts/Roboto.ttf")?
        .with_width(IMAGE_SIZE)?
        .with_height(IMAGE_SIZE)
}

async fn handler_fn(
    s3_client: &aws_sdk_s3::Client,
    bucket_name: &str,
    event: LambdaEvent<CognitoEventUserPoolsPostConfirmation>,
) -> Result<CognitoEventUserPoolsPostConfirmation, Error> {
    let user_attributes_iter = &mut event.payload.request.user_attributes.iter();

    let user_sub = user_attributes_iter.find(|attr| attr.0 == "sub");
    let user_name = user_attributes_iter.find(|attr| attr.0 == "name");

    if let (Some(sub_value), Some(name_value)) = (user_sub, user_name) {
        let avatar = avatar(name_value.1);

        if let Ok(avatar_result) = avatar {
            let image_key = format!("user/{}.png", sub_value.1);
            let image_content = avatar_result.draw().to_vec();
            let image_buffer: Option<ImageBuffer<Rgba<u8>, Vec<u8>>> =
                ImageBuffer::from_vec(IMAGE_SIZE, IMAGE_SIZE, image_content);

            if let Some(buf_result) = image_buffer {
                let mut bytes: Vec<u8> = Vec::new();
                buf_result.write_to(&mut Cursor::new(&mut bytes), ImageOutputFormat::Png)?;

                s3_client
                    .put_object()
                    .bucket(bucket_name)
                    .key(image_key)
                    .content_type("image/png")
                    .body(ByteStream::from(bytes))
                    .send()
                    .await?;
            }
        }
    }

    Ok(event.payload)
}
