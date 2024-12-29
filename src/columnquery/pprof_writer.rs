use datafusion::arrow::array::RecordBatch;

use crate::{pprofpb, profile};
use std::collections::HashMap;

struct MappingKey {
    size: u64,
    offset: u64,
    buildid_or_file: i64,
}

struct FunctionKey {
    start_line: i64,
    name: i64,
    system_name: i64,
    file_name: i64,
}

pub(crate) struct PprofWriter {
    res: pprofpb::Profile,
    mapping_by_key: HashMap<MappingKey, u64>,
    function_by_key: HashMap<FunctionKey, u64>,
    location_by_key: HashMap<String, u64>,
    sample_by_key: HashMap<String, i32>,
    buf: Vec<u8>,
    string_table_index: HashMap<String, usize>,
}

impl PprofWriter {
    pub(crate) fn new(meta: profile::Meta) -> Self {
        let res = pprofpb::Profile {
            string_table: vec!["".to_string()],
            time_nanos: meta.timestamp * 1000000,
            duration_nanos: meta.duration,
            period: meta.period,
            ..Default::default()
        };

        let mut w = Self {
            res,
            mapping_by_key: HashMap::new(),
            function_by_key: HashMap::new(),
            sample_by_key: HashMap::new(),
            location_by_key: HashMap::new(),
            string_table_index: HashMap::from([("".into(), 0)]),
            buf: Vec::with_capacity(4096),
        };

        w.res.period_type = Some(pprofpb::ValueType {
            r#type: w.string(meta.period_type.type_) as i64,
            unit: w.string(meta.period_type.unit) as i64,
        });

        w.res.sample_type = vec![pprofpb::ValueType {
            r#type: w.string(meta.sample_type.type_) as i64,
            unit: w.string(meta.sample_type.unit) as i64,
        }];

        w
    }

    fn string(&mut self, s: String) -> usize {
        if let Some(idx) = self.string_table_index.get(&s) {
            return *idx;
        }

        let idx = self.res.string_table.len();
        self.res.string_table.push(s.clone());
        self.string_table_index.insert(s, idx);
        idx
    }

    pub(crate) fn write_record(&mut self, record: RecordBatch) -> anyhow::Result<()> {
        todo!()
    }

    pub(crate) fn finish(&mut self) -> Result<super::ColumnQueryResponse, anyhow::Error> {
        todo!()
    }
}
