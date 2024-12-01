use crate::{metapb, profile::executableinfo};
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

#[derive(Debug, Clone, Default)]
pub struct MappingLocations {
    mapping: metapb::Mapping,
    locations: HashMap<u64, super::Location>,
}

pub fn symbolize_locations(
    locations: &[Vec<u8>],
    symbolizer: Arc<crate::symbolizer::Symbolizer>,
) -> anyhow::Result<Vec<super::Location>> {
    let mut index_map: HashMap<String, HashMap<executableinfo::Mapping, MappingLocations>> =
        HashMap::new();
    let mut result_locations = Vec::new();

    for loc in locations {
        let decoded_location = crate::profile::PprofLocations::decode(loc)?;

        // Early continue for invalid locations
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

        let mapping_locations = index_map
            .entry(decoded_location.build_id.clone())
            .or_default()
            .entry(mapping)
            .or_insert_with(|| MappingLocations {
                mapping: metapb::Mapping {
                    build_id: decoded_location.build_id.clone(),
                    file: decoded_location.file_name.clone(),
                    start: decoded_location.mapping_memory_start,
                    limit: decoded_location.mapping_memory_end,
                    offset: decoded_location.mapping_file_offset,
                    ..Default::default()
                },
                locations: HashMap::new(),
            });

        mapping_locations
            .locations
            .entry(decoded_location.address)
            .or_insert_with(|| super::Location {
                address: decoded_location.address,
                mapping: Some(mapping_locations.mapping.clone()),
                ..Default::default()
            });
    }

    // Symbolization phase
    for (build_id, mapping_addr_index) in index_map {
        let mut sym_req = crate::symbolizer::SymbolizationRequest {
            build_id,
            mappings: Vec::new(),
        };

        for (_, mapping_locations) in mapping_addr_index {
            let locations: Vec<super::Location> =
                mapping_locations.locations.values().cloned().collect();

            sym_req
                .mappings
                .push(crate::symbolizer::SymbolizationRequestMappingAddrs { locations });
        }

        // Mutate the request in-place
        symbolizer.symbolize(&mut sym_req)?;

        // Extract symbolized locations from the mutated request
        for mapping in sym_req.mappings {
            result_locations.extend(mapping.locations);
        }
    }

    Ok(result_locations)
}
