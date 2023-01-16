/*---------- Imports ----------*/
use aws_config::SdkConfig;
use aws_lambda_events::dynamodb::EventRecord;

pub async fn handler(record: &EventRecord, _config: &SdkConfig) {
    println!("Record: {:#?}", record);
}
