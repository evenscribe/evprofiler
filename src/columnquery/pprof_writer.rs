use crate::{pprofpb, profile};
use byteorder::ByteOrder;
use datafusion::arrow::{
    array::{Array, AsArray, GenericByteArray, RecordBatch},
    datatypes::{GenericBinaryType, Int64Type, UInt32Type, UInt64Type},
};
use std::{collections::HashMap, sync::Arc};

use super::record_reader::RecordReader;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct MappingKey {
    size: u64,
    offset: u64,
    buildid_or_file: i64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
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
        let record_reader = RecordReader::new(&record);
        let transpositions = self.transpose(&record_reader);

        for i in 0..record.num_rows() {
            self.sample(&record_reader, &transpositions, i)
        }

        Ok(())
    }

    pub(crate) fn finish(&mut self) -> anyhow::Result<pprofpb::Profile> {
        Ok(std::mem::take(&mut self.res))
    }

    fn transpose(&mut self, rr: &RecordReader) -> PprofTranspositions {
        let mapping_file = Arc::clone(
            &rr.mapping_file_col
                .as_dictionary_opt::<UInt32Type>()
                .unwrap()
                .values(),
        );
        let mapping_file = mapping_file.as_binary::<i32>();

        let mapping_buildid = Arc::clone(
            &rr.mapping_buildid_col
                .as_dictionary_opt::<UInt32Type>()
                .unwrap()
                .values(),
        );
        let mapping_buildid = mapping_buildid.as_binary::<i32>();

        let line_function_name = Arc::clone(
            &rr.line_function_name_col
                .as_dictionary_opt::<UInt32Type>()
                .unwrap()
                .values(),
        );
        let line_function_name = line_function_name.as_binary::<i32>();

        let line_function_systemname = Arc::clone(
            &rr.line_function_systemname_col
                .as_dictionary_opt::<UInt32Type>()
                .unwrap()
                .values(),
        );
        let line_function_systemname = line_function_systemname.as_binary::<i32>();

        let line_function_filename = Arc::clone(
            &rr.line_function_systemname_col
                .as_dictionary_opt::<UInt32Type>()
                .unwrap()
                .values(),
        );
        let line_function_filename = line_function_filename.as_binary::<i32>();

        PprofTranspositions {
            mapping_file: self.transpose_binary_array(mapping_file),
            mapping_buildid: self.transpose_binary_array(mapping_buildid),
            function_name: self.transpose_binary_array(line_function_name),
            function_system_name: self.transpose_binary_array(line_function_systemname),
            functon_filename: self.transpose_binary_array(line_function_filename),
        }
    }

    fn transpose_binary_array(
        &mut self,
        arr: &GenericByteArray<GenericBinaryType<i32>>,
    ) -> Vec<i64> {
        let res_len = arr.len();
        let mut res = Vec::with_capacity(res_len);

        for indx in 0..res_len {
            res.push(self.byte_string(arr.value(indx)));
        }

        res
    }

    fn byte_string(&mut self, buf: &[u8]) -> i64 {
        let s: String = to_string(buf);

        if let Some(indx) = self.string_table_index.get(&s) {
            return *indx as i64;
        }

        let indx = self.res.string_table.len();
        self.res.string_table.push(s.clone());
        self.string_table_index.insert(s, indx);

        indx as i64
    }

    fn sample(
        &mut self,
        record_reader: &RecordReader,
        transpositions: &PprofTranspositions,
        i: usize,
    ) {
        let locations_col = record_reader.locations_col.as_list::<i32>();
        let j = i + locations_col.offset();
        let loc_start = locations_col.offsets()[j] as usize;
        let loc_end = locations_col.offsets()[j + 1] as usize;

        let value_col = record_reader.value_col.as_primitive::<Int64Type>();
        if loc_start != loc_end {
            let mut s = pprofpb::Sample {
                location_id: Vec::with_capacity(loc_end - loc_start),
                ..Default::default()
            };

            for j in loc_start..loc_end {
                if !locations_col.values().is_valid(j) {
                    continue;
                }
                let l = self.location(record_reader, transpositions, j);
                if l != 0 {
                    s.location_id.push(l);
                }
            }

            // There must be at least one location per sample.
            if !s.location_id.is_empty() {
                let (key, _label_num) = self.sample_key(record_reader, transpositions, i, &s);
                let key = to_string(key);

                if let Some(idx) = self.sample_by_key.get(&key) {
                    //TODO: handle is_diff
                    self.res.sample[*idx as usize].value[0] += value_col.value(i);
                    return;
                }

                //TODO: Handle Labels

                s.value.push(value_col.value(i));
                self.res.sample.push(s);
            }
        }
    }

    fn location(
        &mut self,
        record_reader: &RecordReader,
        transpositions: &PprofTranspositions,
        j: usize,
    ) -> u64 {
        let address_col = record_reader.address_col.as_primitive::<UInt64Type>();
        let lines_col = record_reader.lines_col.as_list::<i32>();
        let line_col = record_reader.line_col.as_struct();
        let line_number_col = record_reader.line_number_col.as_primitive::<Int64Type>();

        let j = j + lines_col.offset();
        let line_start = lines_col.offsets()[j] as usize;
        let line_end = lines_col.offsets()[j + 1] as usize;

        let mut loc = pprofpb::Location {
            mapping_id: self.mapping(record_reader, transpositions, j),
            address: address_col.value(j),
            ..Default::default()
        };

        if line_start != line_end {
            loc.line = Vec::with_capacity(line_end - line_start);

            for k in line_start..line_end {
                if line_col.is_valid(k) {
                    let function_id = self.function(record_reader, transpositions, k);
                    loc.line.push(pprofpb::Line {
                        function_id,
                        line: line_number_col.value(k as usize),
                    });
                }
            }
        }

        let key = self.make_location_key(&loc);
        let key = to_string(key);
        if let Some(idx) = self.location_by_key.get(&key) {
            return *idx as u64;
        }
        let id = self.res.location.len() as u64;
        loc.id = id;
        self.res.location.push(loc);
        self.location_by_key.insert(key, id);

        id
    }

    fn sample_key(
        &mut self,
        _record_reader: &RecordReader,
        _transpositions: &PprofTranspositions,
        _i: usize,
        s: &pprofpb::Sample,
    ) -> (&[u8], usize) {
        let label_num = 0;
        // TODO: Handle Labels

        let key = self.get_buf(16 * label_num + 8 * s.location_id.len());

        // TODO: add label columns to key

        let offset = label_num * 16;
        for (k, l) in s.location_id.iter().enumerate() {
            byteorder::BigEndian::write_u64(&mut key[offset + k * 8..], *l);
        }

        (key, label_num)
    }

    fn mapping(
        &mut self,
        record_reader: &RecordReader,
        transpositions: &PprofTranspositions,
        j: usize,
    ) -> u64 {
        if !record_reader.mapping_file_col.is_valid(j) {
            return 0;
        }

        let mapping_start = record_reader.mapping_start_col.as_primitive::<UInt64Type>();
        let mapping_limit = record_reader.mapping_limit_col.as_primitive::<UInt64Type>();
        let mapping_offset = record_reader
            .mapping_offset_col
            .as_primitive::<UInt64Type>();
        let mapping_file = record_reader.mapping_file_col.as_dictionary::<UInt32Type>();
        let mapping_buildid = record_reader
            .mapping_buildid_col
            .as_dictionary::<UInt32Type>();

        let mut mapping = pprofpb::Mapping {
            memory_start: mapping_start.value(j),
            memory_limit: mapping_limit.value(j),
            file_offset: mapping_offset.value(j),
            filename: transpositions.mapping_file[mapping_file.key(j).unwrap()],
            build_id: transpositions.mapping_buildid[mapping_buildid.key(j).unwrap()],
            has_functions: true,
            ..Default::default()
        };

        let key: MappingKey = make_mapping_key(&mapping);

        if let Some(idx) = self.mapping_by_key.get(&key) {
            return *idx as u64;
        }

        mapping.id = self.res.mapping.len() as u64 + 1;
        self.res.mapping.push(mapping);
        self.mapping_by_key.insert(key, mapping.id);

        mapping.id
    }

    fn function(
        &mut self,
        record_reader: &RecordReader,
        transpositions: &PprofTranspositions,
        k: usize,
    ) -> u64 {
        if !record_reader.line_function_name_col.is_valid(k) {
            return 0;
        }

        let function_name = record_reader
            .line_function_name_col
            .as_dictionary::<UInt32Type>();
        let function_systemname = record_reader
            .line_function_systemname_col
            .as_dictionary::<UInt32Type>();
        let function_filename = record_reader
            .line_function_filename_col
            .as_dictionary::<UInt32Type>();
        let function_start_line = record_reader.line_number_col.as_primitive::<Int64Type>();

        let mut f = pprofpb::Function {
            name: transpositions.function_name[function_name.key(k).unwrap()],
            system_name: transpositions.function_system_name[function_systemname.key(k).unwrap()],
            filename: transpositions.functon_filename[function_filename.key(k).unwrap()],
            start_line: function_start_line.value(k),
            ..Default::default()
        };

        let key: FunctionKey = make_function_key(&f);

        if let Some(idx) = self.function_by_key.get(&key) {
            return *idx as u64;
        }

        f.id = self.res.function.len() as u64 + 1;
        self.res.function.push(f);
        self.function_by_key.insert(key, f.id);
        f.id
    }

    fn make_location_key(&mut self, loc: &pprofpb::Location) -> &[u8] {
        if loc.mapping_id != 0 && loc.address != 0 {
            let m = self.res.mapping[loc.mapping_id as usize - 1];
            let addr = loc.address - m.memory_start;

            let key = self.get_buf(16);
            byteorder::BigEndian::write_u64(key, loc.mapping_id);
            byteorder::BigEndian::write_u64(&mut key[8..], addr);
            return key;
        }

        let key = self.get_buf(16 * loc.line.len());
        for (i, l) in loc.line.iter().enumerate() {
            byteorder::BigEndian::write_u64(&mut key[i * 16..], l.function_id);
            byteorder::BigEndian::write_u64(&mut key[(i * 16) + 8..], l.line as u64);
        }
        key
    }

    fn get_buf(&mut self, cap: usize) -> &mut [u8] {
        if self.buf.len() < cap {
            self.buf.resize(cap, 0);
        }
        &mut self.buf[..cap]
    }
}

fn make_function_key(f: &pprofpb::Function) -> FunctionKey {
    FunctionKey {
        start_line: f.start_line,
        name: f.name,
        system_name: f.system_name,
        file_name: f.filename,
    }
}

fn make_mapping_key(mapping: &pprofpb::Mapping) -> MappingKey {
    // Normalize addresses to handle address space randomization.
    // Round up to next 4K boundary to avoid minor discrepancies.
    let map_size_rounding = 0x1000;

    let size = mapping.memory_limit - mapping.memory_start;
    let size = size + map_size_rounding - 1;
    let size = size - (size % map_size_rounding);
    let mut key = MappingKey {
        size,
        offset: mapping.file_offset,
        ..Default::default()
    };

    if mapping.build_id != 0 {
        key.buildid_or_file = mapping.build_id;
    } else if mapping.filename != 0 {
        key.buildid_or_file = mapping.filename;
    } else {
        // A mapping containing neither build ID nor file name is a fake mapping. A
        // key with empty buildIDOrFile is used for fake mappings so that they are
        // treated as the same mapping during merging.
    }

    key
}

fn to_string(buf: &[u8]) -> String {
    if buf.is_empty() {
        return "".to_string();
    } else {
        return String::from_utf8_lossy(buf).to_string();
    }
}

struct PprofTranspositions {
    mapping_file: Vec<i64>,
    mapping_buildid: Vec<i64>,
    function_name: Vec<i64>,
    function_system_name: Vec<i64>,
    functon_filename: Vec<i64>,
}
