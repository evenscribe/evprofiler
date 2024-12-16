use super::profile::NormalizedProfile;
use super::write_raw::NormalizedWriteRawRequest;
use super::{NormalizedSample, POSSIBLE_METADATA_LABELS};
use crate::pprofpb::{Function, Location, Mapping, Profile, Sample};
use crate::profile::{schema, Meta, PprofLocations, ValueType};
use crate::profilestorepb::{ExecutableInfo, WriteRawRequest};
use anyhow::bail;
use arrow2::array::{
    Array, DictionaryArray, Int64Array, ListArray, MutableArray, MutableBinaryArray,
    MutableDictionaryArray, MutableListArray, MutablePrimitiveArray, MutableUtf8Array, TryPush,
};
use arrow2::chunk::Chunk;
use arrow2::datatypes::Schema;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const NANOS_PER_MILLI: i64 = 1_000_000;

pub fn validate_pprof_profile(
    profile: &Profile,
    executable_info: &[ExecutableInfo],
) -> anyhow::Result<()> {
    if let Some(elem) = profile.string_table.first() {
        if !elem.is_empty() {
            bail!("first string table element is expected to be empty");
        }
    }

    let string_table_len = profile.string_table.len();
    let mappings_length = profile.mapping.len();

    for (i, mapping) in profile.mapping.iter().enumerate() {
        if mapping.id != (i + 1) as u64 {
            bail!("mapping id is not sequential");
        }

        if mapping.filename != 0 && mapping.filename > string_table_len as i64 {
            bail!("mapping filename index out of bounds");
        }

        if mapping.build_id != 0 && mapping.build_id > string_table_len as i64 {
            bail!("mapping build_id index out of bounds");
        }
    }

    if executable_info.len() != mappings_length {
        bail!(
            "Profile has {} mappings, but {} executable infos",
            mappings_length,
            executable_info.len()
        );
    }

    let functions_length = profile.function.len();
    for (i, function) in profile.function.iter().enumerate() {
        if function.id != (i + 1) as u64 {
            bail!("function id is not sequential");
        }

        if function.name != 0 && function.name > string_table_len as i64 {
            bail!("function name index out of bounds");
        }

        if function.system_name != 0 && function.system_name > string_table_len as i64 {
            bail!("function system_name index out of bounds");
        }

        if function.filename != 0 && function.filename > string_table_len as i64 {
            bail!("function filename index out of bounds");
        }
    }

    for (i, location) in profile.location.iter().enumerate() {
        if location.id != (i + 1) as u64 {
            bail!("location id is not sequential");
        }

        if location.mapping_id != 0 && location.mapping_id > profile.mapping.len() as u64 {
            bail!("location mapping_id index out of bounds");
        }

        for line in location.line.iter() {
            if line.function_id != 0 && line.function_id > functions_length as u64 {
                bail!("location function_id index out of bounds");
            }
        }
    }

    if profile.sample_type.is_empty() && !profile.sample.is_empty() {
        bail!("profile has samples but no sample_type");
    }

    for (i, sample) in profile.sample.iter().enumerate() {
        if sample.value.len() != profile.sample_type.len() {
            bail!(
                "sample {} has {} values, expected {}",
                i,
                sample.value.len(),
                profile.sample_type.len()
            );
        }

        for (j, location) in sample.location_id.iter().enumerate() {
            if *location == 0 {
                bail!(
                    "sample {} has location_id 0 at index {}. it must be non zero.",
                    i,
                    j
                );
            }

            if *location > profile.location.len() as u64 {
                bail!(
                    "sample {} has location_id {} at index {}. it must be less than {}.",
                    i,
                    location,
                    j,
                    profile.location.len()
                );
            }
        }

        for (j, label) in sample.label.iter().enumerate() {
            if label.key == 0 {
                bail!(
                    "sample {} has label key 0 at index {}. it must be non zero.",
                    i,
                    j
                );
            }

            if label.key > string_table_len as i64 {
                bail!(
                    "sample {} has label key {} at index {}. it must be less than {}.",
                    i,
                    label.key,
                    j,
                    profile.string_table.len()
                );
            }

            if label.str != 0 && label.str > string_table_len as i64 {
                bail!(
                    "sample {} has label str {} at index {}. it must be less than {}.",
                    i,
                    label.str,
                    j,
                    profile.string_table.len()
                );
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
) -> anyhow::Result<Vec<NormalizedProfile>> {
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
                locations: serialize_pprof_stacktrace(
                    sample.location_id.as_slice(),
                    p.location.as_slice(),
                    p.function.as_slice(),
                    p.mapping.as_slice(),
                    p.string_table.as_slice(),
                )?,
                value: sample.value[i],
                label: labels.clone(),
                num_label: num_labels.clone(),
                diff_value: 0,
            });
        }
    }

    Ok(profiles)
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
        if label.num != 0 && !num_labels.contains_key(key) {
            num_labels.insert(key.to_string(), label.num);
        }
    }

    (res_labels, num_labels)
}

fn serialize_pprof_stacktrace(
    ids: &[u64],
    locations: &[Location],
    functions: &[Function],
    mappings: &[Mapping],
    string_table: &[String],
) -> anyhow::Result<Vec<Vec<u8>>> {
    let mut stacktrace = Vec::with_capacity(ids.len());

    for location_id in ids {
        let location = &locations[*location_id as usize - 1];
        let mapping = match location.mapping_id {
            0 => None,
            _ => Some(&mappings[location.mapping_id as usize - 1]),
        };
        stacktrace.push(PprofLocations::new(location, mapping, functions, string_table).encode()?)
    }

    Ok(stacktrace)
}

pub async fn write_raw_request_to_arrow_chunk(
    request: &WriteRawRequest,
) -> anyhow::Result<Chunk<Arc<dyn Array>>> {
    let normalized_request = NormalizedWriteRawRequest::try_from(request)?;

    let mut duration_column = MutablePrimitiveArray::new();
    let mut name_column: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
        MutableDictionaryArray::new();
    let mut period_column = MutablePrimitiveArray::new();
    let mut period_type_column: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
        MutableDictionaryArray::new();
    let mut period_unit_column: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
        MutableDictionaryArray::new();
    let mut sample_type_column: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
        MutableDictionaryArray::new();
    let mut sample_unit_column: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
        MutableDictionaryArray::new();
    let mut stacktrace_column: MutableListArray<i32, MutableBinaryArray<i32>> =
        MutableListArray::new();
    let mut timestamp_column = MutablePrimitiveArray::new();
    let mut value_column = MutablePrimitiveArray::new();

    for series in normalized_request.series.iter() {
        for profiles in series.samples.iter() {
            for p in profiles {
                for ns in p.samples.iter() {
                    duration_column.push(Some(p.meta.duration));
                    name_column.try_push(Some(p.meta.name.clone()))?;
                    period_column.push(Some(p.meta.period));
                    period_type_column.try_push(Some(p.meta.period_type.type_.clone()))?;
                    period_unit_column.try_push(Some(p.meta.period_type.unit.clone()))?;
                    sample_type_column.try_push(Some(p.meta.sample_type.type_.clone()))?;
                    sample_unit_column.try_push(Some(p.meta.sample_type.unit.clone()))?;
                    if ns.locations.is_empty() {
                        stacktrace_column.push_null();
                    } else {
                        let converted_locations: Vec<Option<&[u8]>> = ns
                            .locations
                            .iter()
                            .map(|loc| {
                                if loc.is_empty() {
                                    None
                                } else {
                                    Some(loc.as_slice())
                                }
                            })
                            .collect();
                        stacktrace_column.try_push(Some(converted_locations))?;
                    }
                    timestamp_column.push(Some(p.meta.timestamp));
                    value_column.push(Some(ns.value));
                }
            }
        }
    }

    let mut fields = vec![
        Int64Array::from(duration_column).arced(),
        DictionaryArray::from(name_column).arced(),
        Int64Array::from(period_column).arced(),
        DictionaryArray::from(period_type_column).arced(),
        DictionaryArray::from(period_unit_column).arced(),
        DictionaryArray::from(sample_type_column).arced(),
        DictionaryArray::from(sample_unit_column).arced(),
        ListArray::from(stacktrace_column).arced(),
        Int64Array::from(timestamp_column).arced(),
        Int64Array::from(value_column).arced(),
    ];

    for name in POSSIBLE_METADATA_LABELS {
        let mut arr: MutableDictionaryArray<i32, MutableUtf8Array<i32>> =
            MutableDictionaryArray::new();

        for series in normalized_request.series.iter() {
            if series.labels.contains_key(name) {
                for profiles in series.samples.iter() {
                    for p in profiles {
                        for _ in p.samples.iter() {
                            arr.try_push(Some(series.labels[name].clone()))?;
                        }
                    }
                }
            } else {
                for profiles in series.samples.iter() {
                    for p in profiles {
                        for _ in p.samples.iter() {
                            arr.push_null();
                        }
                    }
                }
            }
        }
        let arr = DictionaryArray::from(arr);
        //log::info!("labels.{}: {:#?}", name, arr);
        fields.push(arr.arced());
    }

    Ok(Chunk::new(fields))
}
