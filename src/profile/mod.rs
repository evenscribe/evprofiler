mod encode;
pub mod schema;

use crate::metapb::{Function, Mapping};
use arrow::record_batch::RecordBatch;
pub use encode::encode_pprof_location;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LocationLine {
    pub line: i64,
    pub function: Option<Function>,
}

#[derive(Debug, Clone)]
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
