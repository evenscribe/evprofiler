mod debuginfod;
mod metadata;

use crate::debuginfopb::debuginfo::Source;
use crate::debuginfopb::debuginfo_service_server::DebuginfoService;
use crate::debuginfopb::{
    self, BuildIdType, InitiateUploadRequest, InitiateUploadResponse, MarkUploadFinishedRequest,
    MarkUploadFinishedResponse, ShouldInitiateUploadRequest, ShouldInitiateUploadResponse,
    UploadRequest, UploadResponse,
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use debuginfod::DebugInfod;
use metadata::MetadataStore;
use std::sync::{Arc, Mutex};
use tonic::{async_trait, Response, Status};

use self::debuginfopb::debuginfo_upload::State;
use self::debuginfopb::DebuginfoUpload;

const REASON_DEBUGINFO_IN_DEBUGINFOD: &str =
    "Debuginfo exists in debuginfod, therefore no upload is necessary.";
const REASON_FIRST_TIME_SEEN: &str = "First time we see this Build ID, and it does not exist in debuginfod, therefore please upload!";
const REASON_UPLOAD_STALE: &str =
    "A previous upload was started but not finished and is now stale, so it can be retried.";
const REASON_UPLOAD_IN_PROGRESS: &str =
    "A previous upload is still in-progress and not stale yet (only stale uploads can be retried).";
const REASON_DEBUGINFO_ALREADY_EXISTS: &str =
    "Debuginfo already exists and is not marked as invalid, therefore no new upload is needed.";
const REASON_DEBUGINFO_ALREADY_EXISTS_BUT_FORCED: &str = "Debuginfo already exists and is not marked as invalid, therefore wouldn't have accepted a new upload, but accepting it because it's requested to be forced.";
const REASON_DEBUGINFO_INVALID: &str = "Debuginfo already exists but is marked as invalid, therefore a new upload is needed. Hash the debuginfo and initiate the upload.";
const REASON_DEBUGINFO_EQUAL: &str = "Debuginfo already exists and is marked as invalid, but the proposed hash is the same as the one already available, therefore the upload is not accepted as it would result in the same invalid debuginfos.";
const REASON_DEBUGINFO_NOT_EQUAL: &str =
    "Debuginfo already exists but is marked as invalid, therefore a new upload will be accepted.";
const REASON_DEBUGINFOD_SOURCE: &str = "Debuginfo is available from debuginfod already and not marked as invalid, therefore no new upload is needed.";
const REASON_DEBUGINFOD_INVALID: &str = "Debuginfo is available from debuginfod already but is marked as invalid, therefore a new upload is needed.";

pub struct DebuginfoStore {
    metadata: Arc<Mutex<MetadataStore>>,
    debuginfod: Arc<Mutex<DebugInfod>>,
    max_upload_duration: Duration,
}

fn validate_input(id: &str) -> Result<(), Status> {
    if id.len() <= 2 {
        return Err(Status::invalid_argument("unexpectedly short input"));
    }

    Ok(())
}

#[async_trait]
impl DebuginfoService for DebuginfoStore {
    /// Upload ingests debug info for a given build_id
    async fn upload(
        &self,
        request: tonic::Request<tonic::Streaming<UploadRequest>>,
    ) -> std::result::Result<Response<UploadResponse>, Status> {
        Ok(Response::new(UploadResponse::default()))
    }

    // ShouldInitiateUpload returns whether an upload should be initiated for the
    // given build ID. Checking if an upload should even be initiated allows the
    // parca-agent to avoid extracting debuginfos unnecessarily from a binary.
    async fn should_initiate_upload(
        &self,
        request: tonic::Request<ShouldInitiateUploadRequest>,
    ) -> std::result::Result<Response<ShouldInitiateUploadResponse>, Status> {
        let request = request.into_inner();
        let build_id = &request.build_id;

        let _ = validate_input(&build_id)?;
        let req_type = &request.r#type();

        let mut metadata = match self.metadata.lock() {
            Ok(metadata) => metadata,
            Err(_) => return Err(Status::internal("Failed to lock metadata")),
        };

        let mut debuginfod = match self.debuginfod.lock() {
            Ok(debuginfod) => debuginfod,
            Err(_) => return Err(Status::internal("Failed to lock debuginfod")),
        };

        match metadata.fetch(&build_id, req_type) {
            Some(debuginfo) => match Source::from_i32(debuginfo.source) {
                Some(Source::Upload) => match &debuginfo.upload {
                    Some(upload) => match State::from_i32(upload.state) {
                        Some(State::Uploading) => {
                            if self.is_upload_stale(&upload) {
                                return Ok(Response::new(ShouldInitiateUploadResponse {
                                    should_initiate_upload: true,
                                    reason: REASON_UPLOAD_STALE.into(),
                                }));
                            }
                            return Ok(Response::new(ShouldInitiateUploadResponse {
                                should_initiate_upload: false,
                                reason: REASON_UPLOAD_IN_PROGRESS.into(),
                            }));
                        }
                        Some(State::Uploaded) => {
                            if debuginfo.quality.is_none()
                                || debuginfo.quality.unwrap().not_valid_elf
                            {
                                if request.force {
                                    return Ok(Response::new(ShouldInitiateUploadResponse {
                                        should_initiate_upload: true,
                                        reason: REASON_DEBUGINFO_ALREADY_EXISTS_BUT_FORCED.into(),
                                    }));
                                }
                                return Ok(Response::new(ShouldInitiateUploadResponse {
                                    should_initiate_upload: false,
                                    reason: REASON_DEBUGINFO_ALREADY_EXISTS.into(),
                                }));
                            }

                            if request.hash.is_empty() {
                                return Ok(Response::new(ShouldInitiateUploadResponse {
                                    should_initiate_upload: true,
                                    reason: REASON_DEBUGINFO_INVALID.into(),
                                }));
                            }

                            match &debuginfo.upload {
                                Some(upload) => {
                                    if upload.hash.eq(&request.hash) {
                                        return Ok(Response::new(ShouldInitiateUploadResponse {
                                            should_initiate_upload: false,
                                            reason: REASON_DEBUGINFO_EQUAL.into(),
                                        }));
                                    }
                                }
                                None => {
                                    return Ok(Response::new(ShouldInitiateUploadResponse {
                                        should_initiate_upload: true,
                                        reason: REASON_DEBUGINFO_INVALID.into(),
                                    }));
                                }
                            }

                            return Ok(Response::new(ShouldInitiateUploadResponse {
                                should_initiate_upload: true,
                                reason: REASON_DEBUGINFO_NOT_EQUAL.into(),
                            }));
                        }

                        _ => {
                            return Err(Status::internal(
                                "inconssistent metadata: unknown upload state",
                            ));
                        }
                    },
                    None => {
                        return Err(Status::internal(
                            "inconssistent metadata: unknown upload state",
                        ));
                    }
                },

                Some(Source::Debuginfod) => {
                    if debuginfo.quality.is_none() || debuginfo.quality.unwrap().not_valid_elf {
                        return Ok(Response::new(ShouldInitiateUploadResponse {
                            should_initiate_upload: true,
                            reason: REASON_DEBUGINFOD_SOURCE.into(),
                        }));
                    }

                    return Ok(Response::new(ShouldInitiateUploadResponse {
                        should_initiate_upload: true,
                        reason: REASON_DEBUGINFOD_INVALID.into(),
                    }));
                }

                _ => {
                    return Err(Status::internal("inconssistent metadata: unknown source"));
                }
            },
            None => {
                // First time we see this Build ID.
                let build_id_type = request.build_id_type();
                if build_id_type == BuildIdType::Gnu
                    || build_id_type == BuildIdType::UnknownUnspecified
                {
                    if debuginfod.exists(&build_id) {
                        metadata.mark(&build_id, req_type);

                        return Ok(Response::new(ShouldInitiateUploadResponse {
                            should_initiate_upload: false,
                            reason: REASON_DEBUGINFO_IN_DEBUGINFOD.into(),
                        }));
                    }
                } else {
                    return Ok(Response::new(ShouldInitiateUploadResponse {
                        should_initiate_upload: true,
                        reason: REASON_FIRST_TIME_SEEN.into(),
                    }));
                }
            }
        };

        Ok(Response::new(ShouldInitiateUploadResponse::default()))
    }
    /// InitiateUpload returns a strategy and information to upload debug info for a given build_id.
    async fn initiate_upload(
        &self,
        request: tonic::Request<InitiateUploadRequest>,
    ) -> std::result::Result<Response<InitiateUploadResponse>, Status> {
        Ok(Response::new(InitiateUploadResponse::default()))
    }
    /// MarkUploadFinished marks the upload as finished for a given build_id.
    async fn mark_upload_finished(
        &self,
        request: tonic::Request<MarkUploadFinishedRequest>,
    ) -> std::result::Result<Response<MarkUploadFinishedResponse>, Status> {
        Ok(Response::new(MarkUploadFinishedResponse::default()))
    }
}

impl DebuginfoStore {
    pub fn default() -> Self {
        Self {
            metadata: Arc::new(Mutex::new(MetadataStore::default())),
            debuginfod: Arc::new(Mutex::new(DebugInfod::default())),
            max_upload_duration: Duration::minutes(15),
        }
    }

    fn is_upload_stale(&self, upload: &DebuginfoUpload) -> bool {
        match upload.started_at {
            Some(ts) => {
                let started_at = Utc
                    .timestamp_opt(ts.seconds, ts.nanos as u32)
                    .earliest()
                    .unwrap_or(Utc::now());

                started_at + (self.max_upload_duration + Duration::minutes(2)) < self.time_now()
            }
            None => false,
        }
    }

    fn time_now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
