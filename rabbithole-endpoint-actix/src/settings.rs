use rabbithole::model::version::JsonApiVersion;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct JsonApiSettings {
    pub version: JsonApiVersion,
}
