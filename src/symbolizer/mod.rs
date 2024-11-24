mod liner;
mod normalize;

use crate::debuginfo_store::DebuginfoFetcher;
use crate::profile::LocationLine;
use crate::symbols::{elfutils, Demangler};
use crate::{debuginfo_store::MetadataStore, profile::Location};
use crate::{
    debuginfopb::{self, DebuginfoQuality, DebuginfoType},
    profile::executableinfo::{ExecutableInfo, Mapping},
};
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

use self::debuginfopb::Debuginfo;

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
        todo!()
    }

    fn set(
        &self,
        build_id: &str,
        addr: &NormalizedAddress,
        clone: Vec<LocationLine>,
    ) -> Result<(), Status> {
        todo!()
    }
}

pub struct Symbolizer {
    demangler: Demangler,
    cache: SymbolizerCache,
    metadata: Arc<Mutex<MetadataStore>>,
    fetcher: DebuginfoFetcher,
    temp_dir: PathBuf,
}

pub struct SymbolizationRequestMappingAddrs<'a> {
    /// This slice is used to store the symbolization result directly.
    locations: &'a mut [Location],
}

pub struct SymbolizationRequest {
    build_id: String,
    mappings: Vec<SymbolizationRequestMappingAddrs<'static>>,
}

struct ElfDebugInfo<'a> {
    pub(crate) target_path: PathBuf,
    pub(crate) e: object::File<'a>,
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

    pub fn symbolize(&self, request: SymbolizationRequest) -> Result<(), Status> {
        log::log!(
            log::Level::Debug,
            "Symbolizing request for build_id: {}",
            request.build_id
        );

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

        let l = Liner::new(
            &request.build_id,
            &elf_debug_info,
            &self.cache,
            &self.demangler,
        );

        let ei = match ExecutableInfo::try_from(&elf_debug_info.e) {
            Ok(ei) => ei,
            Err(e) => {
                return Err(Status::internal(format!(
                    "Failed to get ExecutableInfo: {}",
                    e
                )))
            }
        };

        for mapping in request.mappings {
            for location in mapping.locations {
                let mapping = match &location.mapping {
                    Some(mapping) => mapping,
                    None => return Err(Status::internal("Provided Empty Mappings.")),
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

                location.lines = l.liner.unwrap().pc_to_lines(addr)?;
            }
        }

        Ok(())
    }

    fn check_quality(quality: &Option<DebuginfoQuality>) -> Result<(), Status> {
        if let Some(q) = quality {
            if q.not_valid_elf {
                return Err(Status::not_found("Not a valid ELF file"));
            }

            if !q.has_dwarf && !q.has_go_pclntab && !(q.has_symtab || q.has_dynsym) {
                return Err(Status::not_found("Check debuginfo quality."));
            }
        }
        Ok(())
    }

    fn validate_source(dbginfo: &debuginfopb::Debuginfo) -> Result<(), Status> {
        match dbginfo.source() {
            debuginfopb::debuginfo::Source::Upload => {
                let upload = dbginfo
                    .upload
                    .as_ref()
                    .ok_or_else(|| Status::not_found("Debuginfo not uploaded yet"))?;

                if upload.state() != debuginfopb::debuginfo_upload::State::Uploaded {
                    return Err(Status::not_found("Debuginfo not uploaded yet"));
                }
            }
            debuginfopb::debuginfo::Source::Debuginfod => (),
            _ => return Err(Status::not_found("Debuginfo not found")),
        }
        Ok(())
    }

    fn lock_metadata(&self) -> Result<MutexGuard<MetadataStore>, Status> {
        self.metadata
            .lock()
            .map_err(|_| Status::internal("Failed to lock metadata store"))
    }

    fn create_and_write_temp_file(&self, data: &[u8], build_id: &str) -> Result<PathBuf, Status> {
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

    fn update_quality(&self, build_id: &str, quality: DebuginfoQuality) -> Result<(), Status> {
        let mut metadata = self.lock_metadata()?;
        metadata.set_quality(build_id, &quality, &DebuginfoType::DebuginfoUnspecified)
    }

    fn get_debug_info<'a>(
        &self,
        build_id: &str,
        dbginfo: &mut Debuginfo,
        in_data: &'a [u8],
    ) -> Result<ElfDebugInfo<'a>, Status> {
        let target_path = self.create_and_write_temp_file(&in_data, build_id)?;

        // Parse ELF file
        let file = object::File::parse(&*in_data).map_err(|e| {
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

            dbginfo.quality = Some(quality.clone());
            self.update_quality(&dbginfo.build_id, quality)?;

            // Validate the new quality
            Self::check_quality(&dbginfo.quality)?;
        }

        Ok(ElfDebugInfo {
            target_path,
            e: file,
            quality: dbginfo.quality.clone(),
        })
    }
}
