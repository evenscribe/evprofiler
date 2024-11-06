use super::profile::NormalizedProfile;
use super::NormalizedSample;
use crate::pprofpb::{Function, Location, Mapping, Profile, Sample};
use crate::profile::{encode_pprof_location, Meta, ValueType};
use crate::profilestorepb::ExecutableInfo;
use std::collections::{HashMap, HashSet};
use std::result::Result;
use tonic::Status;

const NANOS_PER_MILLI: i64 = 1_000_000;

pub fn validate_pprof_profile(
    profile: &Profile,
    executable_info: &[ExecutableInfo],
) -> Result<(), Status> {
    match profile.string_table.first() {
        Some(s) => {
            if !s.is_empty() {
                return Err(Status::invalid_argument(format!(
                    "first item in string table is expected to be empty string, but it is {}",
                    s
                )));
            }
        }
        None => {}
    };

    for (i, mapping) in profile.mapping.iter().enumerate() {
        if mapping.id != (i + 1) as u64 {
            return Err(Status::invalid_argument("mapping id is not sequential"));
        }

        if mapping.filename != 0 && mapping.filename > profile.string_table.len() as i64 {
            return Err(Status::invalid_argument(
                "mapping filename index out of bounds",
            ));
        }

        if mapping.build_id != 0 && mapping.build_id > profile.string_table.len() as i64 {
            return Err(Status::invalid_argument(
                "mapping build_id index out of bounds",
            ));
        }
    }

    if executable_info.len() != profile.mapping.len() {
        return Err(Status::invalid_argument(format!(
            "Profile has {} mappings, but {} executable infos",
            profile.mapping.len(),
            executable_info.len(),
        )));
    }

    for (i, function) in profile.function.iter().enumerate() {
        if function.id != (i + 1) as u64 {
            return Err(Status::invalid_argument("function id is not sequential"));
        }

        if function.name != 0 && function.name > profile.string_table.len() as i64 {
            return Err(Status::invalid_argument(
                "function name index out of bounds",
            ));
        }

        if function.system_name != 0 && function.system_name > profile.string_table.len() as i64 {
            return Err(Status::invalid_argument(
                "function system_name index out of bounds",
            ));
        }

        if function.filename != 0 && function.filename > profile.string_table.len() as i64 {
            return Err(Status::invalid_argument(
                "function filename index out of bounds",
            ));
        }
    }

    for (i, location) in profile.location.iter().enumerate() {
        if location.id != (i + 1) as u64 {
            return Err(Status::invalid_argument("location id is not sequential"));
        }

        if location.mapping_id != 0 && location.mapping_id > profile.mapping.len() as u64 {
            return Err(Status::invalid_argument(
                "location mapping_id index out of bounds",
            ));
        }

        for line in location.line.iter() {
            if line.function_id != 0 && line.function_id > profile.function.len() as u64 {
                return Err(Status::invalid_argument(format!(
                    "location {} has invalid function_id {}",
                    location.id, line.function_id
                )));
            }
        }
    }

    if profile.sample_type.len() != 0 && profile.sample.len() != 0 {
        return Err(Status::invalid_argument("missing sample type information"));
    }

    for (i, sample) in profile.sample.iter().enumerate() {
        if sample.value.len() != profile.sample_type.len() {
            return Err(Status::invalid_argument(format!(
                "sample {} has {} values, expected {}",
                i,
                sample.value.len(),
                profile.sample_type.len()
            )));
        }

        for (j, location) in sample.location_id.iter().enumerate() {
            if *location == 0 {
                return Err(Status::invalid_argument(format!(
                    "sample {} has location_id 0 at index {}. it must be non zero.",
                    i, j
                )));
            }

            if *location > profile.location.len() as u64 {
                return Err(Status::invalid_argument(format!(
                    "sample {} has location_id {} at index {}. it must be less than {}.",
                    i,
                    location,
                    j,
                    profile.location.len()
                )));
            }
        }

        for (j, label) in sample.label.iter().enumerate() {
            if label.key == 0 {
                return Err(Status::invalid_argument(format!(
                    "sample {} has label key 0 at index {}. it must be non zero.",
                    i, j
                )));
            }

            if label.key > profile.string_table.len() as i64 {
                return Err(Status::invalid_argument(format!(
                    "sample {} has label key {} at index {}. it must be less than {}.",
                    i,
                    label.key,
                    j,
                    profile.string_table.len()
                )));
            }

            if label.str != 0 && label.str > profile.string_table.len() as i64 {
                return Err(Status::invalid_argument(format!(
                    "sample {} has label str {} at index {}. it must be less than {}.",
                    i,
                    label.str,
                    j,
                    profile.string_table.len()
                )));
            }
        }
    }

    Ok(())
}

pub fn label_names_from_profile(
    _: &HashMap<String, String>,
    string_table: &[String],
    samples: &[Sample],
    all_label_names: &mut HashSet<String>,
) {
    let mut labels: HashSet<&str> = HashSet::new();

    for sample in samples.iter() {
        for label in sample.label.iter() {
            if label.str == 0 {
                continue;
            }
            let key = &string_table[label.key as usize];
            if !labels.contains(key.as_str()) {
                labels.insert(key.as_str());
            }
        }
    }

    for label in labels {
        all_label_names.insert(label.to_string());
    }
}

pub fn normalize_pprof(
    name: &str,
    taken_label_names: &HashMap<String, String>,
    p: &Profile,
) -> Vec<NormalizedProfile> {
    let mut profiles: Vec<NormalizedProfile> = Vec::with_capacity(p.sample_type.len());

    for i in 0..p.sample_type.len() {
        let np: NormalizedProfile = NormalizedProfile::new(
            Vec::with_capacity(p.sample.len()),
            meta_from_pprof(p, name, i),
        );
        profiles.push(np);
    }

    for sample in p.sample.iter() {
        let (labels, num_labels) = labels_from_sample(
            taken_label_names,
            p.string_table.as_slice(),
            sample.label.as_slice(),
        );

        for (i, value) in sample.value.iter().enumerate() {
            if *value == 0 {
                continue;
            }

            profiles[i].samples.push(NormalizedSample {
                locations: serialize_stacktrace(
                    sample.location_id.as_slice(),
                    p.location.as_slice(),
                    p.function.as_slice(),
                    p.mapping.as_slice(),
                    p.string_table.as_slice(),
                ),
                value: sample.value[i],
                label: labels.clone(),
                num_label: num_labels.clone(),
                diff_value: 0,
            });
        }
    }

    profiles
}

fn meta_from_pprof(p: &Profile, name: &str, sample_index: usize) -> Meta {
    let period_type = match p.period_type {
        Some(pt) => ValueType {
            type_: p.string_table[pt.r#type as usize].clone(),
            unit: p.string_table[pt.unit as usize].clone(),
        },
        None => ValueType {
            type_: "".to_string(),
            unit: "".to_string(),
        },
    };

    let sample_type = match p.sample_type.get(sample_index) {
        Some(st) => ValueType {
            type_: p.string_table[st.r#type as usize].clone(),
            unit: p.string_table[st.unit as usize].clone(),
        },
        None => ValueType {
            type_: "".to_string(),
            unit: "".to_string(),
        },
    };

    Meta {
        name: name.to_string(),
        timestamp: p.time_nanos / NANOS_PER_MILLI,
        duration: p.duration_nanos,
        period: p.period,
        period_type,
        sample_type,
    }
}

pub fn labels_from_sample(
    _: &HashMap<String, String>,
    string_table: &[String],
    plabels: &[crate::pprofpb::Label],
) -> (HashMap<String, String>, HashMap<String, i64>) {
    let mut labels: HashMap<String, Vec<String>> = HashMap::new();
    let mut label_names = vec![];

    for label in plabels.iter() {
        if label.str == 0 {
            continue;
        }

        let key = &string_table[label.key as usize];
        if !labels.contains_key(key) {
            labels.insert(key.to_string(), vec![]);
            label_names.push(key.to_string());
        }
        labels
            .get_mut(key)
            .unwrap()
            .push(string_table[label.str as usize].clone());
    }

    label_names.sort();

    let mut res_labels: HashMap<String, String> = HashMap::new();

    for label_name in label_names.iter() {
        res_labels.insert(
            label_name.clone(),
            labels.get(label_name).unwrap().first().unwrap().into(),
        );
    }

    let mut num_labels: HashMap<String, i64> = HashMap::new();

    for label in plabels.iter() {
        let key = &string_table[label.key as usize];
        if label.num != 0 {
            if !num_labels.contains_key(key) {
                num_labels.insert(key.to_string(), label.num);
            }
        }
    }

    (res_labels, num_labels)
}

fn serialize_stacktrace(
    ids: &[u64],
    locations: &[Location],
    functions: &[Function],
    mappings: &[Mapping],
    string_table: &[String],
) -> Vec<Vec<u8>> {
    let mut stacktrace = Vec::with_capacity(ids.len());

    for location_id in ids {
        let location = &locations[*location_id as usize - 1];
        let mapping = match location.mapping_id {
            0 => &Mapping {
                id: 0,
                memory_start: 0,
                memory_limit: 0,
                file_offset: 0,
                filename: 0,
                build_id: 0,
                has_functions: false,
                has_filenames: false,
                has_line_numbers: false,
                has_inline_frames: false,
            },
            _ => &mappings[location.mapping_id as usize - 1],
        };
        stacktrace.push(encode_pprof_location(
            location,
            mapping,
            functions,
            string_table,
        ))
    }

    stacktrace
}
