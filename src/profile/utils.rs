use crate::{metapb, profile::executableinfo, symbolizer};
use datafusion::arrow::{
    array::{Array, FixedSizeBinaryArray, GenericByteArray},
    datatypes::GenericBinaryType,
};
use std::{collections::HashMap, sync::Arc};

// #[derive(Debug, Clone, Default)]
// pub struct MappingLocations {
//     mapping: metapb::Mapping,
//     locations: HashMap<u64, super::Location>,
// }
//
// #[derive(Debug, Clone, Default)]
// pub struct IndexValue {
//     inner_map: HashMap<executableinfo::Mapping, MappingLocations>,
// }
//
// pub fn symbolize_locations(
//     locations: &[Vec<u8>],
//     symbolizer: Arc<crate::symbolizer::Symbolizer>,
// ) -> anyhow::Result<Vec<super::Location>> {
//     let mut index_map: HashMap<String, IndexValue> = HashMap::new();
//     let mut count = 0;
//
//     for loc in locations {
//         let decoded_location = crate::profile::PprofLocations::decode(loc)?;
//         if decoded_location.address == 0
//             || decoded_location.build_id.is_empty()
//             || decoded_location.number_of_lines > 0
//         {
//             continue;
//         }
//
//         let indx_value = index_map
//             .entry(decoded_location.build_id.clone())
//             .or_insert(IndexValue::default());
//
//         let mapping = executableinfo::Mapping {
//             start: decoded_location.mapping_memory_start,
//             end: decoded_location.mapping_memory_end,
//             offset: decoded_location.mapping_file_offset,
//             file: decoded_location.file_name.clone(),
//         };
//
//         let ml = indx_value
//             .inner_map
//             .entry(mapping)
//             .or_insert(MappingLocations {
//                 mapping: metapb::Mapping {
//                     build_id: decoded_location.build_id.clone(),
//                     file: decoded_location.file_name.clone(),
//                     start: decoded_location.mapping_memory_start,
//                     limit: decoded_location.mapping_memory_end,
//                     offset: decoded_location.mapping_file_offset,
//                     ..Default::default()
//                 },
//                 locations: HashMap::new(),
//             });
//
//         let _ = ml
//             .locations
//             .entry(decoded_location.address)
//             .or_insert_with(|| {
//                 let loc = crate::profile::Location {
//                     address: decoded_location.address,
//                     mapping: Some(ml.mapping.clone()),
//                     ..Default::default()
//                 };
//                 count += 1;
//                 loc
//             });
//     }
//
//     let res = vec![];
//
//     for (build_id, mapping_addr_indx) in index_map.iter() {
//         let mut sym_req = crate::symbolizer::SymbolizationRequest {
//             build_id: build_id.clone(),
//             mappings: vec![],
//         };
//
//         for (_, mapping_locations) in mapping_addr_indx.inner_map.iter() {
//             let mut locs = Vec::with_capacity(mapping_locations.locations.len());
//             for (_, loc) in mapping_locations.locations.iter() {
//                 locs.push(loc.clone());
//             }
//             sym_req
//                 .mappings
//                 .push(crate::symbolizer::SymbolizationRequestMappingAddrs { locations: locs })
//         }
//
//         let _ = symbolizer.symbolize(&mut sym_req)?;
//     }
//
//     Ok(res)
// }

#[derive(Debug, Default)]
pub struct MappingLocations<'a> {
    mapping: metapb::Mapping,
    locations: HashMap<u64, &'a super::Location>,
}

pub async fn symbolize_locations(
    locations: &GenericByteArray<GenericBinaryType<i32>>,
    symbolizer: Arc<symbolizer::Symbolizer>,
) -> anyhow::Result<Vec<Option<super::Location>>> {
    // Pre-allocate result vector
    let mut result_locations = Vec::with_capacity(locations.len());

    // Create a single map to group locations by build_id and mapping
    let mut symbolization_groups: HashMap<
        (String, executableinfo::Mapping),
        (metapb::Mapping, Vec<(usize, super::Location)>),
    > = HashMap::new();

    // First pass: group locations and fill result vector
    for (idx, loc) in locations.iter().enumerate() {
        result_locations.push(None);

        if loc.is_none() {
            continue;
        }

        let decoded_location = match crate::profile::PprofLocations::decode(loc.unwrap()) {
            Ok(loc) => loc,
            Err(e) => {
                continue;
            }
        };

        // Skip invalid locations
        if decoded_location.address == 0
            || decoded_location.build_id.is_empty()
            || decoded_location.number_of_lines > 0
        {
            continue;
        }

        let mapping = executableinfo::Mapping {
            start: decoded_location.mapping_memory_start,
            end: decoded_location.mapping_memory_end,
            offset: decoded_location.mapping_file_offset,
            file: decoded_location.file_name.clone(),
        };

        let key = (decoded_location.build_id.clone(), mapping.clone());
        let group = symbolization_groups.entry(key).or_insert_with(|| {
            (
                metapb::Mapping {
                    build_id: decoded_location.build_id.clone(),
                    file: decoded_location.file_name.clone(),
                    start: decoded_location.mapping_memory_start,
                    limit: decoded_location.mapping_memory_end,
                    offset: decoded_location.mapping_file_offset,
                    ..Default::default()
                },
                Vec::new(),
            )
        });

        let location = super::Location {
            address: decoded_location.address,
            mapping: Some(group.0.clone()),
            ..Default::default()
        };

        group.1.push((idx, location));
    }

    // Symbolization phase
    for ((build_id, _), (_, locations_with_indices)) in symbolization_groups.iter_mut() {
        let mut locations: Vec<&mut super::Location> = locations_with_indices
            .iter_mut()
            .map(|(_, loc)| loc)
            .collect();

        // Then create SymbolizationRequestMappingAddrs with a slice from the Vec
        let mut sym_req = symbolizer::SymbolizationRequest {
            build_id: build_id.clone(),
            mappings: vec![symbolizer::SymbolizationRequestMappingAddrs {
                locations: locations.as_mut_slice(),
            }],
        };

        let _ = symbolizer.symbolize(&mut sym_req).await;

        // Update result_locations directly from the symbolized locations
        for (idx, loc) in locations_with_indices.iter() {
            if let Some(result) = result_locations.get_mut(*idx) {
                *result = Some(loc.clone());
            }
        }
    }

    Ok(result_locations)
}
