use serde_json::{map::Map, Value};
use std::{
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
#[derive(Debug)]
pub enum IoEvent {
    FetchInput,
}
