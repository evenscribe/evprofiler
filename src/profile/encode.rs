use crate::{
    metapb::Function,
    pprofpb::{Location, Mapping},
};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PprofLocations {
    pub address: u64,
    pub number_of_lines: usize,
    pub build_id: String,
    pub file_name: String,
    pub mapping_memory_start: u64,
    pub mapping_memory_end: u64,
    pub mapping_file_offset: u64,
    pub functions: Vec<Function>,
}

impl PprofLocations {
    pub fn new(
        location: &Location,
        mapping: Option<&Mapping>,
        functions: &[crate::pprofpb::Function],
        string_table: &[String],
    ) -> PprofLocations {
        let mut build_id = String::new();
        let mut file_name = String::new();
        let mut mapping_memory_start = 0;
        let mut mapping_memory_end = 0;
        let mut mapping_file_offset = 0;

        if let Some(mapping) = mapping {
            build_id = string_table[mapping.build_id as usize].clone();
            file_name = string_table[mapping.filename as usize].clone();
            mapping_memory_start = mapping.memory_start;
            mapping_memory_end = mapping.memory_limit;
            mapping_file_offset = mapping.file_offset;
        }

        let mut funcs: Vec<Function> = vec![];

        for line in location.line.iter() {
            let start_line = line.line;
            if line.function_id == 0 {
                funcs.push(Function {
                    start_line,
                    ..Default::default()
                })
            } else {
                let pf = &functions[line.function_id as usize - 1];
                funcs.push(Function {
                    start_line,
                    name: if pf.name > 0 {
                        string_table[pf.name as usize].clone()
                    } else {
                        String::new()
                    },
                    system_name: if pf.system_name > 0 {
                        string_table[pf.system_name as usize].clone()
                    } else {
                        String::new()
                    },
                    filename: if pf.filename > 0 {
                        string_table[pf.filename as usize].clone()
                    } else {
                        String::new()
                    },
                    ..Default::default()
                })
            }
        }

        PprofLocations {
            number_of_lines: location.line.len(),
            address: location.address,
            build_id,
            file_name,
            mapping_memory_start,
            mapping_memory_end,
            mapping_file_offset,
            functions: funcs,
        }
    }

    pub fn encode(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bincode::serialize(&self)?)
    }

    pub fn decode(data: &[u8]) -> anyhow::Result<PprofLocations> {
        Ok(bincode::deserialize(data)?)
    }
}

// pub fn encode_pprof_location(
//     location: &Location,
//     mapping: &Mapping,
//     functions: &[Function],
//     string_table: &[String],
// ) -> Vec<u8> {
// let mut buf = Vec::with_capacity(serialized_pprof_location_size(
//     location,
//     mapping,
//     functions,
//     string_table,
// ));
// write_uvarint(&mut buf, location.address);
// write_uvarint(&mut buf, location.line.len() as u64);
//
// if mapping.id == 0 {
//     buf.push(0x0);
// } else {
//     buf.push(0x1);
//     let build_id = match mapping.build_id {
//         0 => "",
//         _ => &string_table[mapping.build_id as usize],
//     };
//     write_string(&mut buf, build_id);
//
//     let filename = match mapping.filename {
//         0 => "",
//         _ => &string_table[mapping.filename as usize],
//     };
//     write_string(&mut buf, filename);
//     write_uvarint(&mut buf, mapping.memory_start);
//     write_uvarint(&mut buf, mapping.memory_limit - mapping.memory_start);
//     write_uvarint(&mut buf, mapping.file_offset);
// }
//
// for line in location.line.iter() {
//     write_uvarint(&mut buf, line.line as u64);
//
//     if line.function_id != 0 {
//         buf.push(0x1);
//
//         let f = &functions[line.function_id as usize - 1];
//         write_uvarint(&mut buf, f.start_line as u64);
//
//         let name = match f.name {
//             0 => "",
//             _ => &string_table[f.name as usize],
//         };
//         write_string(&mut buf, name);
//
//         let system_name = match f.system_name {
//             0 => "",
//             _ => &string_table[f.system_name as usize],
//         };
//         write_string(&mut buf, system_name);
//
//         let filename = match f.filename {
//             0 => "",
//             _ => &string_table[f.filename as usize],
//         };
//         write_string(&mut buf, filename);
//     } else {
//         buf.push(0x0);
//     }
// }
//
// buf
// }

// fn serialized_pprof_location_size(
//     location: &Location,
//     mapping: &Mapping,
//     functions: &[Function],
//     string_table: &[String],
// ) -> usize {
//     let mut size = uvarint_size(location.address);
//     size += 1;
//
//     size += uvarint_size(location.line.len() as u64);
//
//     if mapping.id != 0 {
//         let build_id = match mapping.build_id {
//             0 => "",
//             _ => &string_table[mapping.build_id as usize],
//         };
//         size += uvarint_size(build_id.len() as u64) + build_id.len();
//
//         let filename = match mapping.filename {
//             0 => "",
//             _ => &string_table[mapping.filename as usize],
//         };
//         size += uvarint_size(filename.len() as u64) + filename.len();
//         size += uvarint_size(mapping.memory_start);
//         size += uvarint_size(mapping.memory_limit - mapping.memory_start);
//         size += uvarint_size(mapping.file_offset);
//     }
//
//     for line in location.line.iter() {
//         size += uvarint_size(line.line as u64);
//         size += 1;
//
//         if line.function_id != 0 {
//             let f = &functions[line.function_id as usize - 1];
//             size += uvarint_size(f.start_line as u64);
//
//             let name = match f.name {
//                 0 => "",
//                 _ => &string_table[f.name as usize],
//             };
//             size += uvarint_size(name.len() as u64) + name.len();
//
//             let system_name = match f.system_name {
//                 0 => "",
//                 _ => &string_table[f.system_name as usize],
//             };
//             size += uvarint_size(system_name.len() as u64) + system_name.len();
//
//             let filename = match f.filename {
//                 0 => "",
//                 _ => &string_table[f.filename as usize],
//             };
//             size += uvarint_size(filename.len() as u64) + filename.len();
//         }
//     }
//
//     size
// }
