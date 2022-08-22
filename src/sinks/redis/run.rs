use super::StreamStrategy;
use crate::{model::Event, pipelining::StageReceiver, utils::Utils, Error};
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;

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
        let result: Result<(), _>;

        if stream.eq("cip25asset") {
            let parsed_cip25 = &parsed_json["cip25_asset"];
            let asset = parsed_cip25["asset"].to_string();
            let description = parsed_cip25["description"].to_string();
            let image = parsed_cip25["image"].to_string();
            let media_type = parsed_cip25["media_type"].to_string();
            let policy = parsed_cip25["policy"].to_string();
            let name = parsed_cip25["name"].to_string();
            let raw_json = json!(parsed_cip25["raw_json"]).to_string();

             result = redis::cmd("HSET")
            .arg(format!("{}:{}:{}", stream, policy, asset))
            .arg("policy").arg(policy)
            .arg("asset").arg(asset)
            .arg("name").arg(name)
            .arg("image").arg(image)
            .arg("media_type").arg(media_type)
            .arg("description").arg(description)
            .arg("raw_json").arg(raw_json)
            .query(conn);
        } 
        else 
        {
            let result = redis::cmd("HSET")
            .arg(format!("{}:{}", stream, key))
            .arg("json_text").arg(json!(event).to_string())
            .query(conn);
        }


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

    Ok(())
}
