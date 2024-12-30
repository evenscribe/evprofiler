use self::debuginfopb::{debuginfo::Source, debuginfo_upload, DebuginfoUpload};
use crate::debuginfopb::{self, Debuginfo, DebuginfoType};
use anyhow::bail;
use chrono::{DateTime, Utc};
use moka::sync::Cache;
use prost_types::Timestamp;

#[derive(Debug)]
pub struct MetadataStore {
    pub store: Cache<String, Debuginfo>,
}

impl Default for MetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            store: Cache::new(10_000),
        }
    }

    pub fn with_store(store: Cache<String, Debuginfo>) -> Self {
        Self { store }
    }

    pub fn fetch(&self, build_id: &str, req_type: &DebuginfoType) -> Option<Debuginfo> {
        let path = Self::get_object_path(build_id, req_type);
        self.store.get(&path)
    }

    fn get_object_path(build_id: &str, req_type: &DebuginfoType) -> String {
        match req_type {
            DebuginfoType::Executable => format!("{}/executable.metadata", build_id),
            DebuginfoType::Sources => format!("{}/sources.metadata", build_id),
            _ => format!("{}/metadata", build_id),
        }
    }

    pub fn set_quality(
        &self,
        build_id: &str,
        quality: &debuginfopb::DebuginfoQuality,
        req_type: &DebuginfoType,
    ) -> anyhow::Result<()> {
        let path = Self::get_object_path(build_id, req_type);
        let mut entry = match self.store.get(&path) {
            Some(e) => e,
            None => {
                bail!("Debuginfo not found");
            }
        };

        entry.quality = Some(*quality);
        self.store.insert(path, entry);
        Ok(())
    }

    pub fn mark_as_debuginfod_source(
        &self,
        servers: Vec<String>,
        build_id: &str,
        req_type: &DebuginfoType,
    ) -> anyhow::Result<()> {
        self.write(Debuginfo {
            build_id: build_id.to_string(),
            r#type: (*req_type).into(),
            source: Source::Debuginfod.into(),
            upload: None,
            quality: None,
            debuginfod_servers: servers,
        })
    }

    pub fn mark_as_uploading(
        &self,
        build_id: &str,
        upload_id: &str,
        hash: &str,
        req_type: &DebuginfoType,
        started_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        self.write(Debuginfo {
            build_id: build_id.to_string(),
            r#type: (*req_type).into(),
            source: Source::Upload.into(),
            upload: Some(DebuginfoUpload {
                id: upload_id.to_string(),
                hash: hash.to_string(),
                started_at: Some(Timestamp {
                    seconds: started_at.timestamp(),
                    nanos: started_at.timestamp_subsec_nanos() as i32,
                }),
                finished_at: None,
                state: debuginfo_upload::State::Uploading.into(),
            }),
            quality: None,
            debuginfod_servers: vec![],
        })
    }

    pub fn mark_as_uploaded(
        &self,
        build_id: &str,
        upload_id: &str,
        req_type: &DebuginfoType,
        finished_at: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        let debug_info = match self.fetch(build_id, req_type) {
            Some(d) => d,
            None => bail!("Debuginfo not found"),
        };

        let debug_info_upload = match &debug_info.upload {
            Some(diu) => diu,
            None => {
                bail!("Debuginfo is not in uploading state");
            }
        };

        if debug_info_upload.id != upload_id {
            bail!("Debuginfo mismatched upload id");
        }

        let mut debug_info = debug_info.clone();
        let mut debug_info_upload = debug_info_upload.clone();
        debug_info_upload.set_state(debuginfo_upload::State::Uploaded);
        debug_info_upload.finished_at = Some(Timestamp {
            seconds: finished_at.timestamp(),
            nanos: finished_at.timestamp_subsec_nanos() as i32,
        });
        debug_info.upload = Some(debug_info_upload);

        self.write(debug_info)
    }

    pub fn write(&self, debuginfo: Debuginfo) -> anyhow::Result<()> {
        if debuginfo.build_id.is_empty() {
            bail!("build_id is empty. REQUIRED to write debuginfo metadata");
        }

        let debuginfo_type = match DebuginfoType::try_from(debuginfo.r#type) {
            Ok(t) => t,
            Err(_) => bail!("Invalid debuginfo type"),
        };

        let path = Self::get_object_path(&debuginfo.build_id, &debuginfo_type);
        self.store.insert(path, debuginfo);
        Ok(())
    }
}
