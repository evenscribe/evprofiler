mod encode;
pub mod executableinfo;
pub mod schema;
mod utils;

use crate::metapb::{Function, Mapping};
use arrow::record_batch::RecordBatch;
pub use encode::PprofLocations;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
pub use utils::symbolize_locations;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationLine {
    pub line: i64,
    pub function: Option<Function>,
}

impl LocationLine {
    pub fn decode(decoded: &[u8]) -> anyhow::Result<LocationLine> {
        Ok(bincode::deserialize(decoded)?)
    }

    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(self)?)
    }
}

#[derive(Debug, Default, Clone)]
pub struct Location {
    pub id: String,
    pub address: u64,
    pub is_folded: bool,
    pub mapping: Option<Mapping>,
    pub lines: Vec<LocationLine>,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct NumLabel {
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone)]
pub struct SymbolizedSample {
    pub locations: Vec<Location>,
    pub value: i64,
    pub diff_value: i64,
    pub label: HashMap<String, String>,
    pub num_label: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
pub struct NormalizedSample {
    pub stacktrace_id: String,
    pub value: i64,
    pub diff_value: i64,
    pub label: HashMap<String, String>,
    pub num_label: HashMap<String, i64>,
}

#[derive(Debug, Clone)]
pub struct Profile {
    pub samples: Vec<RecordBatch>,
    pub meta: Meta,
}

#[derive(Debug, Clone)]
pub struct OldProfile {
    pub meta: Meta,
    pub samples: Vec<SymbolizedSample>,
}

#[derive(Debug, Clone)]
pub struct NormalizedProfile {
    pub samples: Vec<NormalizedSample>,
    pub meta: Meta,
}

#[derive(Debug, Clone)]
pub struct ValueType {
    pub type_: String,
    pub unit: String,
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub name: String,
    pub period_type: ValueType,
    pub sample_type: ValueType,
    pub timestamp: i64,
    pub duration: i64,
    pub period: i64,
}
