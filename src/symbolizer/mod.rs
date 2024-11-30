pub mod liner;
pub mod normalize;

use self::debuginfopb::Debuginfo;
use crate::debuginfo_store::DebuginfoFetcher;
use crate::profile::LocationLine;
use crate::symbols::{elfutils, Demangler};
use crate::{debuginfo_store::MetadataStore, profile::Location};
use crate::{
    debuginfopb::{self, DebuginfoQuality, DebuginfoType},
    profile::executableinfo::{ExecutableInfo, Mapping},
};
use anyhow::{bail, Context};
use liner::Liner;
use normalize::NormalizedAddress;
use std::io::Write;
use std::path::PathBuf;
use std::sync::MutexGuard;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tonic::Status;

#[derive(Default, Debug, Clone)]
pub struct SymbolizerCache {
    pub(crate) c: HashMap<String, String>,
}

impl SymbolizerCache {
    fn get(
        &self,
        build_id: &str,
        addr: &NormalizedAddress,
    ) -> Result<Option<Vec<LocationLine>>, Status> {
        Ok(None)
    }

    fn set(
        &self,
        build_id: &str,
        addr: &NormalizedAddress,
        clone: Vec<LocationLine>,
    ) -> Result<(), Status> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct Symbolizer {
    pub(crate) demangler: Demangler,
    cache: SymbolizerCache,
    metadata: Arc<Mutex<MetadataStore>>,
    fetcher: DebuginfoFetcher,
    temp_dir: PathBuf,
}

#[derive(Debug)]
pub struct SymbolizationRequestMappingAddrs {
    /// This slice is used to store the symbolization result directly.
    pub locations: Vec<Location>,
}

#[derive(Debug)]
pub struct SymbolizationRequest {
    pub build_id: String,
    pub mappings: Vec<SymbolizationRequestMappingAddrs>,
}

#[derive(Debug)]
pub struct ElfDebugInfo<'data> {
    pub(crate) target_path: PathBuf,
    pub(crate) e: object::File<'data>,
    pub(crate) quality: Option<DebuginfoQuality>,
}

impl Symbolizer {
    pub fn new(metadata: Arc<Mutex<MetadataStore>>, fetcher: DebuginfoFetcher) -> Self {
        Self {
            demangler: Demangler::new(false),
            cache: SymbolizerCache::default(),
            metadata,
            fetcher,
            temp_dir: PathBuf::from("/tmp"),
        }
    }

    pub fn symbolize(&self, request: &mut SymbolizationRequest) -> anyhow::Result<()> {
        log::debug!("Symbolizing request for build_id: {}", request.build_id);

        let build_id = &request.build_id;

        let mut dbginfo = {
            let metadata = self.lock_metadata()?;
            metadata
                .fetch(build_id, &DebuginfoType::DebuginfoUnspecified)
                .ok_or_else(|| {
                    Status::not_found(format!("Debuginfo for build_id {} not found", build_id))
                })?
                .clone()
        };

        // Validate existing quality if present
        Self::check_quality(&dbginfo.quality)?;

        // Validate source
        Self::validate_source(&dbginfo)?;

        let raw_data = self.fetcher.fetch_raw_elf(&dbginfo)?;
        let elf_debug_info = self.get_debug_info(&request.build_id, &mut dbginfo, &raw_data)?;
        log::warn!("elf_debug_info: {:#?}", elf_debug_info);

        let mut l = Liner::new(
            &request.build_id,
            &elf_debug_info,
            &self.cache,
            &self.demangler,
        );

        let ei = ExecutableInfo::try_from(&elf_debug_info.e)?;

        for mapping in request.mappings.iter_mut() {
            for location in mapping.locations.iter_mut() {
                let mapping = match &location.mapping {
                    Some(mapping) => mapping,
                    None => bail!("Mapping not found"),
                };
                let addr = NormalizedAddress::try_new(
                    location.address,
                    &ei,
                    &Mapping {
                        start: mapping.start,
                        end: mapping.limit,
                        offset: mapping.offset,
                        file: String::new(),
                    },
                )?;
                location.lines = l.pc_to_lines(addr)?;
            }
        }

        Ok(())
    }

    fn check_quality(quality: &Option<DebuginfoQuality>) -> anyhow::Result<()> {
        if let Some(q) = quality {
            if q.not_valid_elf {
                bail!("Not a valid ELF file");
            }

            if !(q.has_dwarf || q.has_go_pclntab || q.has_symtab || q.has_dynsym) {
                bail!("Trying to Symbolize but it has none of the quality evprofiler needs. Check debuginfo quality: {:?}", q);
            }
        }
        Ok(())
    }

    fn validate_source(dbginfo: &debuginfopb::Debuginfo) -> anyhow::Result<()> {
        match dbginfo.source() {
            debuginfopb::debuginfo::Source::Upload => {
                let upload = dbginfo
                    .upload
                    .as_ref()
                    .with_context(|| "debug info not uploaded yet")?;

                if upload.state() != debuginfopb::debuginfo_upload::State::Uploaded {
                    bail!("Debuginfo not uploaded yet");
                }
            }
            debuginfopb::debuginfo::Source::Debuginfod => (),
            _ => bail!("Invalid or unsupported source"),
        }
        Ok(())
    }

    fn lock_metadata(&self) -> anyhow::Result<MutexGuard<MetadataStore>> {
        let m = self
            .metadata
            .lock()
            .map_err(|_| Status::internal("Failed to lock metadata store"))?;
        Ok(m)
    }

    fn create_and_write_temp_file(&self, data: &[u8], build_id: &str) -> anyhow::Result<PathBuf> {
        let mut tmp_file = tempfile::NamedTempFile::new_in(&self.temp_dir)
            .map_err(|e| Status::internal(format!("Failed to create temporary file: {}", e)))?;

        tmp_file
            .write_all(data)
            .map_err(|e| Status::internal(format!("Failed to write to temporary file: {}", e)))?;

        tmp_file
            .flush()
            .map_err(|e| Status::internal(format!("Failed to flush temporary file: {e}")))?;

        let target_path = self.temp_dir.join(build_id);
        tmp_file
            .persist(&target_path)
            .map_err(|e| Status::internal(format!("Failed to persist temporary file: {}", e)))?;

        Ok(target_path)
    }

    fn update_quality(&self, build_id: &str, quality: DebuginfoQuality) -> anyhow::Result<()> {
        let mut metadata = self.lock_metadata()?;
        let _ = metadata.set_quality(build_id, &quality, &DebuginfoType::DebuginfoUnspecified)?;
        Ok(())
    }

    fn get_debug_info<'a>(
        &self,
        build_id: &str,
        dbginfo: &mut Debuginfo,
        in_data: &'a [u8],
    ) -> anyhow::Result<ElfDebugInfo<'a>> {
        let target_path = self.create_and_write_temp_file(in_data, build_id)?;

        // Parse ELF file
        let file = object::File::parse(in_data).map_err(|e| {
            // Update quality on parse failure
            let quality = DebuginfoQuality {
                not_valid_elf: true,
                has_dwarf: false,
                has_go_pclntab: false,
                has_symtab: false,
                has_dynsym: false,
            };
            let _ = self.update_quality(build_id, quality);
            Status::internal(format!("Failed to parse ELF file: {}", e))
        })?;

        // Update quality if not present
        if dbginfo.quality.is_none() {
            let quality = DebuginfoQuality {
                not_valid_elf: false,
                has_dwarf: elfutils::has_dwarf(&file),
                has_go_pclntab: elfutils::has_go_pcln_tab(&file),
                has_symtab: elfutils::has_symtab(&file),
                has_dynsym: elfutils::has_dynsym(&file),
            };

            dbginfo.quality = Some(quality);
            self.update_quality(&dbginfo.build_id, quality)?;

            // Validate the new quality
            Self::check_quality(&dbginfo.quality)?;
        }

        Ok(ElfDebugInfo {
            target_path,
            e: file,
            quality: dbginfo.quality,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{debuginfo_store, metapb, profile};

    use super::*;

    #[test]
    fn symbolization_test() {
        let metadata_store = Arc::new(Mutex::new(debuginfo_store::MetadataStore::new()));
        let debuginfod = Arc::new(Mutex::new(debuginfo_store::DebugInfod::default()));
        let bucket: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::from(HashMap::new()));
        let symbolizer = Arc::new(Symbolizer::new(
            Arc::clone(&metadata_store),
            DebuginfoFetcher::new(Arc::clone(&bucket), Arc::clone(&debuginfod)),
        ));

        let mapping = metapb::Mapping {
            start: 4194304,
            limit: 4603904,
            build_id: "2d6912fd3dd64542f6f6294f4bf9cb6c265b3085".into(),
            ..Default::default()
        };

        let location = profile::Location {
            mapping: Some(mapping.clone()),
            address: 0x463781,
            ..Default::default()
        };
    }
}
