use crate::pprofpb::{Function, Location, Mapping};

fn write_uvarint(buf: &mut Vec<u8>, value: u64) {
    let mut value = value;
    while value >= 0x80 {
        buf.push((value as u8) | 0x80);
        value >>= 7;
    }
    buf.push(value as u8);
}

fn write_string(buf: &mut Vec<u8>, value: &str) {
    write_uvarint(buf, value.len() as u64);
    buf.extend_from_slice(value.as_bytes());
}

fn uvarint_size(value: u64) -> usize {
    match value {
        0 => 1,
        v => {
            // Calculate how many bytes are needed to represent the value
            // Each byte in uvarint encoding can hold 7 bits of data
            // We add 7 to round up the division
            (64 - v.leading_zeros() as usize + 6) / 7
        }
    }
}

pub fn encode_pprof_location(
    location: &Location,
    mapping: &Mapping,
    functions: &[Function],
    string_table: &[String],
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(serialized_pprof_location_size(
        location,
        mapping,
        functions,
        string_table,
    ));
    let _ = write_uvarint(&mut buf, location.address);
    let _ = write_uvarint(&mut buf, location.line.len() as u64);

    if mapping.id == 0 {
        buf.push(0x0);
    } else {
        buf.push(0x1);
        let build_id = match mapping.build_id {
            0 => "",
            _ => &string_table[mapping.build_id as usize],
        };
        let _ = write_string(&mut buf, build_id);

        let filename = match mapping.filename {
            0 => "",
            _ => &string_table[mapping.filename as usize],
        };
        let _ = write_string(&mut buf, filename);
        let _ = write_uvarint(&mut buf, mapping.memory_start);
        let _ = write_uvarint(&mut buf, mapping.memory_limit - mapping.memory_start);
        let _ = write_uvarint(&mut buf, mapping.file_offset);
    }

    for line in location.line.iter() {
        let _ = write_uvarint(&mut buf, line.line as u64);

        if line.function_id != 0 {
            buf.push(0x1);

            let f = &functions[line.function_id as usize - 1];
            let _ = write_uvarint(&mut buf, f.start_line as u64);

            let name = match f.name {
                0 => "",
                _ => &string_table[f.name as usize],
            };
            let _ = write_string(&mut buf, name);

            let system_name = match f.system_name {
                0 => "",
                _ => &string_table[f.system_name as usize],
            };
            let _ = write_string(&mut buf, system_name);

            let filename = match f.filename {
                0 => "",
                _ => &string_table[f.filename as usize],
            };
            let _ = write_string(&mut buf, filename);
        } else {
            buf.push(0x0);
        }
    }

    buf
}

fn serialized_pprof_location_size(
    location: &Location,
    mapping: &Mapping,
    functions: &[Function],
    string_table: &[String],
) -> usize {
    let mut size = uvarint_size(location.address);
    size += 1;

    size += uvarint_size(location.line.len() as u64);

    if mapping.id != 0 {
        let build_id = match mapping.build_id {
            0 => "",
            _ => &string_table[mapping.build_id as usize],
        };
        size += uvarint_size(build_id.len() as u64) + build_id.len();

        let filename = match mapping.filename {
            0 => "",
            _ => &string_table[mapping.filename as usize],
        };
        size += uvarint_size(filename.len() as u64) + filename.len();
        size += uvarint_size(mapping.memory_start);
        size += uvarint_size(mapping.memory_limit - mapping.memory_start);
        size += uvarint_size(mapping.file_offset);
    }

    for line in location.line.iter() {
        size += uvarint_size(line.line as u64);
        size += 1;

        if line.function_id != 0 {
            let f = &functions[line.function_id as usize - 1];
            size += uvarint_size(f.start_line as u64);

            let name = match f.name {
                0 => "",
                _ => &string_table[f.name as usize],
            };
            size += uvarint_size(name.len() as u64) + name.len();

            let system_name = match f.system_name {
                0 => "",
                _ => &string_table[f.system_name as usize],
            };
            size += uvarint_size(system_name.len() as u64) + system_name.len();

            let filename = match f.filename {
                0 => "",
                _ => &string_table[f.filename as usize],
            };
            size += uvarint_size(filename.len() as u64) + filename.len();
        }
    }

    size
}
