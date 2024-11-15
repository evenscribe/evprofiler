mod debuginfod;
mod metadata;

use crate::debuginfopb::debuginfo::Source;
use crate::debuginfopb::debuginfo_service_server::DebuginfoService;
use crate::debuginfopb::Debuginfo;
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
        validate_input(&request.build_id)?;

        let mut metadata = self
            .metadata
            .lock()
            .map_err(|_| Status::internal("Failed to lock metadata"))?;
        let mut debuginfod = self
            .debuginfod
            .lock()
            .map_err(|_| Status::internal("Failed to lock debuginfod"))?;

        match metadata.fetch(&request.build_id, &request.r#type()) {
            Some(debuginfo) => self.handle_existing_debuginfo(&request, &debuginfo),
            None => self.handle_new_build_id(&request, &mut metadata, &mut debuginfod),
        }
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

    fn handle_existing_debuginfo(
        &self,
        request: &ShouldInitiateUploadRequest,
        debuginfo: &Debuginfo,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        match Source::from_i32(debuginfo.source) {
            Some(Source::Upload) => self.handle_upload_source(request, debuginfo),
            Some(Source::Debuginfod) => self.handle_debuginfod_source(debuginfo),
            _ => Err(Status::internal("Inconsistent metadata: unknown source")),
        }
    }

    fn handle_upload_source(
        &self,
        request: &ShouldInitiateUploadRequest,
        debuginfo: &Debuginfo,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        let upload = debuginfo
            .upload
            .as_ref()
            .ok_or_else(|| Status::internal("Inconsistent metadata: missing upload info"))?;

        match State::from_i32(upload.state) {
            Some(State::Uploading) => self.handle_uploading_state(upload),
            Some(State::Uploaded) => self.handle_uploaded_state(request, debuginfo),
            _ => Err(Status::internal(
                "Inconsistent metadata: unknown upload state",
            )),
        }
    }

    fn handle_uploading_state(
        &self,
        upload: &DebuginfoUpload,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        if self.is_upload_stale(upload) {
            Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_UPLOAD_STALE.into(),
            }))
        } else {
            Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: false,
                reason: REASON_UPLOAD_IN_PROGRESS.into(),
            }))
        }
    }

    fn handle_uploaded_state(
        &self,
        request: &ShouldInitiateUploadRequest,
        debuginfo: &Debuginfo,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        if !self.is_valid_elf(debuginfo) {
            return self.handle_invalid_elf(request);
        }

        if request.hash.is_empty() {
            return Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_DEBUGINFO_INVALID.into(),
            }));
        }

        self.compare_hash(request, debuginfo)
    }

    fn is_valid_elf(&self, debuginfo: &Debuginfo) -> bool {
        debuginfo
            .quality
            .as_ref()
            .map_or(false, |q| !q.not_valid_elf)
    }

    fn handle_invalid_elf(
        &self,
        request: &ShouldInitiateUploadRequest,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        Ok(Response::new(ShouldInitiateUploadResponse {
            should_initiate_upload: request.force,
            reason: if request.force {
                REASON_DEBUGINFO_ALREADY_EXISTS_BUT_FORCED.into()
            } else {
                REASON_DEBUGINFO_ALREADY_EXISTS.into()
            },
        }))
    }

    fn compare_hash(
        &self,
        request: &ShouldInitiateUploadRequest,
        debuginfo: &Debuginfo,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        match &debuginfo.upload {
            Some(upload) if upload.hash.eq(&request.hash) => {
                Ok(Response::new(ShouldInitiateUploadResponse {
                    should_initiate_upload: false,
                    reason: REASON_DEBUGINFO_EQUAL.into(),
                }))
            }
            Some(_) => Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_DEBUGINFO_NOT_EQUAL.into(),
            })),
            None => Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_DEBUGINFO_INVALID.into(),
            })),
        }
    }

    fn handle_debuginfod_source(
        &self,
        debuginfo: &Debuginfo,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        Ok(Response::new(ShouldInitiateUploadResponse {
            should_initiate_upload: true,
            reason: if !self.is_valid_elf(debuginfo) {
                REASON_DEBUGINFOD_SOURCE.into()
            } else {
                REASON_DEBUGINFOD_INVALID.into()
            },
        }))
    }

    fn handle_new_build_id(
        &self,
        request: &ShouldInitiateUploadRequest,
        metadata: &mut MetadataStore,
        debuginfod: &mut DebugInfod,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        match request.build_id_type() {
            BuildIdType::Gnu | BuildIdType::UnknownUnspecified
                if debuginfod.exists(&request.build_id) =>
            {
                metadata.mark(&request.build_id, &request.r#type());
                Ok(Response::new(ShouldInitiateUploadResponse {
                    should_initiate_upload: false,
                    reason: REASON_DEBUGINFO_IN_DEBUGINFOD.into(),
                }))
            }
            _ => Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_FIRST_TIME_SEEN.into(),
            })),
        }
    }
}
