use super::StreamStrategy;
use crate::{model::Event, pipelining::StageReceiver, utils::Utils, Error};
use serde_json::json;
use serde_json::Value;
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

/*
        let result: Result<(), _> = redis::cmd("XADD")
            .arg(stream)
            .arg("*")
            .arg(&[("key", key)])
            .arg(&[("json", json!(event).to_string())])
            .query(conn);
*/
/* 
        let result: Result<(), _> = redis::cmd("JSON.SET")
            .arg(key)
            .arg("$")
            .arg(json!(event).to_string())
            .query(conn);
*/

        let parsedCip25: Value = serde_json::from_str(&json!(event).to_string())?;
        let asset = parsedCip25["asset"];
        let description = parsedCip25["description"];
        let image = parsedCip25["image"];
        let media_type = parsedCip25["media_type"];
        let policy = parsedCip25["policy"];
        let name = parsedCip25["name"];
        let raw_json = json!(parsedCip25["raw_json"]).to_string();

        let result: Result<(), _> = redis::cmd("HSET")
        .arg(format!("cip25_asset:{}.{}", policy, asset))
        .arg("policy").arg(policy)
        .arg("asset").arg(asset)
        .arg("name").arg(name)
        .arg("image").arg(image)
        .arg("media_type").arg(media_type)
        .arg("description").arg(description)
        .arg("raw_json").arg(raw_json)
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

    Ok(())
}
