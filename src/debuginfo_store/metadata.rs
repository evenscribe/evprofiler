use self::debuginfopb::{debuginfo::Source, debuginfo_upload, DebuginfoUpload};
use crate::debuginfopb::{self, Debuginfo, DebuginfoType};
use chrono::{DateTime, Utc};
use prost_types::Timestamp;
use std::collections::HashMap;
use tonic::Status;

#[derive(Debug, Default)]
pub struct MetadataStore {
    store: HashMap<String, Debuginfo>,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn fetch(&self, build_id: &str, req_type: &DebuginfoType) -> Option<&Debuginfo> {
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
        &mut self,
        build_id: &str,
        quality: &debuginfopb::DebuginfoQuality,
        req_type: &DebuginfoType,
    ) -> Result<(), Status> {
        Ok(())
    }

    pub fn mark_as_debuginfod_source(
        &mut self,
        servers: Vec<String>,
        build_id: &str,
        req_type: &DebuginfoType,
    ) -> Result<(), Status> {
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
        &mut self,
        build_id: &str,
        upload_id: &str,
        hash: &str,
        req_type: &DebuginfoType,
        started_at: DateTime<Utc>,
    ) -> Result<(), Status> {
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
        &mut self,
        build_id: &str,
        upload_id: &str,
        req_type: &DebuginfoType,
        finished_at: DateTime<Utc>,
    ) -> Result<(), Status> {
        let debug_info = match self.fetch(build_id, req_type) {
            Some(d) => d,
            None => return Err(Status::not_found("Debuginfo not found")),
        };

        let debug_info_upload = match &debug_info.upload {
            Some(diu) => diu,
            None => {
                return Err(Status::invalid_argument(
                    "Debuginfo is not in uploading state",
                ));
            }
        };

        if debug_info_upload.id != upload_id {
            return Err(Status::invalid_argument("Debuginfo mismatched upload id"));
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

    pub fn write(&mut self, debuginfo: Debuginfo) -> Result<(), Status> {
        if debuginfo.build_id.is_empty() {
            return Err(Status::invalid_argument(
                "build_id is empty. REQUIRED to write debuginfo metadata",
            ));
        }

        let debuginfo_type = match DebuginfoType::try_from(debuginfo.r#type) {
            Ok(t) => t,
            Err(_) => return Err(Status::invalid_argument("Invalid debuginfo type")),
        };

        let path = Self::get_object_path(&debuginfo.build_id, &debuginfo_type);
        self.store.insert(path, debuginfo);
        Ok(())
    }
}