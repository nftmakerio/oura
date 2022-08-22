use super::StreamStrategy;
use crate::{model::Event, pipelining::StageReceiver, utils::Utils, Error};
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use hex;

fn key(event: &Event) -> String {
    if let Some(fingerprint) = &event.fingerprint {
        fingerprint.clone()
    } else {
        event.data.clone().to_string().to_lowercase()
    }
}

pub fn producer_loop(
    input: StageReceiver,
    utils: Arc<Utils>,
    conn: &mut redis::Connection,
    stream_strategy: StreamStrategy,
    redis_stream: String,
) -> Result<(), Error> {
    for event in input.iter() {
        let key = key(&event);

        let stream = match stream_strategy {
            StreamStrategy::ByEventType => event.data.clone().to_string().to_lowercase(),
            _ => redis_stream.clone(),
        };

        log::debug!(
            "Stream: {:?}, Key: {:?}, Event: {:?}",
            &stream,
            &key,
            &event
        );

        let parsed_json: Value = serde_json::from_str(&json!(event).to_string()).unwrap();


        if stream.eq("cip25asset") {
            let parsed_cip25 = &parsed_json["cip25_asset"];
            let asset = parsed_cip25["asset"].to_string();
            let asset_hex = parsed_cip25["asset"].to_string();
            let name = parsed_cip25["name"].to_string();
            let description = parsed_cip25["description"].to_string();
            let asset_lc = parsed_cip25["asset"].to_string().to_lowercase();
            let name_lc = parsed_cip25["name"].to_string().to_lowercase();
            let description_lc = parsed_cip25["description"].to_string().to_lowercase();
            let image = parsed_cip25["image"].to_string();
            let media_type = parsed_cip25["media_type"].to_string().to_lowercase();
            let policy = parsed_cip25["policy"].to_string();
            let raw_json = json!(parsed_cip25["raw_json"]).to_string().to_lowercase();

            let context = &parsed_json["context"];
            let timestamp = context["timestamp"].to_string().replace("\"","");


            let mut keyName = format!("{}:{}:{}", stream, policy, hex::encode(asset_hex));
            keyName = keyName.replace("\"","");

            let result: Result<(), _> = redis::cmd("HSET")
            .arg(keyName)
            .arg("policy").arg(policy)
            .arg("asset").arg(asset)
            .arg("name").arg(name)
            .arg("description").arg(description)
            .arg("asset_lc").arg(asset_lc)
            .arg("name_lc").arg(name_lc)
            .arg("description_lc").arg(description_lc)
            .arg("image").arg(image)
            .arg("media_type").arg(media_type)
            .arg("raw_json").arg(raw_json)
            .arg("timestamp").arg(timestamp)
            .query(conn);

            match result {
                Ok(_) => {
                    utils.track_sink_progress(&event);
                }
                Err(err) => {
                    log::error!("error sending message to redis: {}", err);
                    return Err(Box::new(err));
                }
            }
        } 
        else 
        {
            let result: Result<(), _> = redis::cmd("XADD")
            .arg(stream)
            .arg("*")
            .arg(&[(key, json!(event).to_string())])
            .query(conn);

        match result {
            Ok(_) => {
                utils.track_sink_progress(&event);
            }
            Err(err) => {
                log::error!("error sending message to redis: {}", err);
                return Err(Box::new(err));
            }
        }
        }

    }

    Ok(())
}
