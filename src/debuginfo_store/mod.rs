mod debuginfod;
mod metadata;

use self::debuginfopb::debuginfo_upload::State;
use self::debuginfopb::upload_request;
use self::debuginfopb::{DebuginfoType, DebuginfoUpload};
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
use std::collections::HashMap;
use std::result::Result;
use std::sync::{Arc, Mutex};
use tokio_stream::StreamExt;
use tonic::{async_trait, Request, Response, Status, Streaming};

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

pub struct UploadRequestInfo {
    buildid: String,
    upload_id: String,
    debuginfo_type: DebuginfoType,
}

impl TryFrom<upload_request::Data> for UploadRequestInfo {
    type Error = Status;
    fn try_from(data: upload_request::Data) -> Result<Self, Self::Error> {
        match data {
            upload_request::Data::Info(upload_info) => Ok(Self {
                buildid: upload_info.build_id,
                upload_id: upload_info.upload_id,
                debuginfo_type: match DebuginfoType::try_from(upload_info.r#type) {
                    Ok(t) => t,
                    Err(_) => return Err(Status::invalid_argument("Invalid debuginfo type.")),
                },
            }),
            _ => Err(Status::invalid_argument("Invalid data type.")),
        }
    }
}

pub struct DebuginfoStore {
    metadata: Arc<Mutex<MetadataStore>>,
    debuginfod: Arc<Mutex<DebugInfod>>,
    max_upload_duration: Duration,
    bucket: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

#[async_trait]
impl DebuginfoService for DebuginfoStore {
    /// Upload ingests debug info for a given build_id
    async fn upload(
        &self,
        request: Request<Streaming<UploadRequest>>,
    ) -> Result<Response<UploadResponse>, Status> {
        let mut stream = request.into_inner();

        let request = match stream.message().await {
            Ok(Some(msg)) => msg,
            Ok(None) => return Err(Status::invalid_argument("Empty request")),
            Err(e) => {
                return Err(Status::internal(format!(
                    "Failed to receive message: {}",
                    e
                )))
            }
        };

        let data = request
            .data
            .ok_or_else(|| Status::invalid_argument("Missing data"))?;
        let upload_info = UploadRequestInfo::try_from(data)?;
        let _ = self.validate_buildid(&upload_info.buildid)?;

        let dbginfo = {
            let metadata = self
                .metadata
                .lock()
                .map_err(|_| Status::internal("Failed to lock metadata"))?;

            metadata
                .fetch(&upload_info.buildid, &upload_info.debuginfo_type)
                .ok_or_else(|| {
                    Status::failed_precondition(
                "metadata not found, this indicates that the upload was not previously initiated"
            )
                })?
                .clone()
        };

        let upload = dbginfo.upload.ok_or_else(|| {
            Status::invalid_argument(
                "metadata not found, this indicates that the upload was not previously initiated",
            )
        })?;

        if upload.id.ne(&upload_info.upload_id) {
            return Err(Status::failed_precondition(
            "upload metadata not found, this indicates that the upload was not previously initiated"
        ));
        }

        let mut chunks = Vec::new();
        while let Some(req) = stream.next().await {
            let req = req?;
            match req.data {
                Some(upload_request::Data::ChunkData(chunk)) => {
                    chunks.extend(chunk);
                }
                _ => {
                    return Err(Status::invalid_argument(
                        "provided no value or invalid data",
                    ))
                }
            }
        }

        let size = chunks.len() as u64;

        {
            let mut bucket = self
                .bucket
                .lock()
                .map_err(|_| Status::internal("Failed to lock bucket"))?;
            bucket.insert(upload_info.upload_id, chunks);
        }

        Ok(Response::new(UploadResponse {
            build_id: upload_info.buildid,
            size,
        }))
    }

    // ShouldInitiateUpload returns whether an upload should be initiated for the
    // given build ID. Checking if an upload should even be initiated allows the
    // parca-agent to avoid extracting debuginfos unnecessarily from a binary.
    async fn should_initiate_upload(
        &self,
        request: Request<ShouldInitiateUploadRequest>,
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        let request = request.into_inner();
        let _ = self.validate_buildid(&request.build_id)?;

        let debuginfo = {
            let metadata = self
                .metadata
                .lock()
                .map_err(|_| Status::internal("Failed to lock metadata"))?;
            metadata
                .fetch(&request.build_id, &request.r#type())
                .cloned()
        };

        match debuginfo {
            Some(info) => self.handle_existing_debuginfo(&request, &info),
            None => self.handle_new_build_id(&request),
        }
    }

    /// InitiateUpload returns a strategy and information to upload debug info for a given build_id.
    async fn initiate_upload(
        &self,
        request: Request<InitiateUploadRequest>,
    ) -> Result<Response<InitiateUploadResponse>, Status> {
        Ok(Response::new(InitiateUploadResponse::default()))
    }
    /// MarkUploadFinished marks the upload as finished for a given build_id.
    async fn mark_upload_finished(
        &self,
        request: Request<MarkUploadFinishedRequest>,
    ) -> Result<Response<MarkUploadFinishedResponse>, Status> {
        Ok(Response::new(MarkUploadFinishedResponse::default()))
    }
}

impl DebuginfoStore {
    pub fn default() -> Self {
        Self {
            metadata: Arc::new(Mutex::new(MetadataStore::default())),
            debuginfod: Arc::new(Mutex::new(DebugInfod::default())),
            max_upload_duration: Duration::minutes(15),
            bucket: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn validate_buildid(&self, id: &str) -> Result<(), Status> {
        if id.len() <= 2 {
            return Err(Status::invalid_argument("unexpectedly short input"));
        }

        Ok(())
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
        match Source::try_from(debuginfo.source) {
            Ok(Source::Debuginfod) => self.handle_debuginfod_source(debuginfo),
            Ok(Source::Upload) => self.handle_upload_source(request, debuginfo),
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

        match State::try_from(upload.state) {
            Ok(State::Uploading) => self.handle_uploading_state(upload),
            Ok(State::Uploaded) => self.handle_uploaded_state(request, debuginfo),
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
    ) -> Result<Response<ShouldInitiateUploadResponse>, Status> {
        if !matches!(
            request.build_id_type(),
            BuildIdType::Gnu | BuildIdType::UnknownUnspecified
        ) {
            return Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_FIRST_TIME_SEEN.into(),
            }));
        }

        let exists = {
            let mut debuginfod = self
                .debuginfod
                .lock()
                .map_err(|_| Status::internal("Failed to lock debuginfod"))?;
            debuginfod.exists(&request.build_id)
        };

        if exists {
            {
                let mut metadata = self
                    .metadata
                    .lock()
                    .map_err(|_| Status::internal("Failed to lock metadata"))?;
                metadata.mark(&request.build_id, &request.r#type());
            }

            Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: false,
                reason: REASON_DEBUGINFO_IN_DEBUGINFOD.into(),
            }))
        } else {
            Ok(Response::new(ShouldInitiateUploadResponse {
                should_initiate_upload: true,
                reason: REASON_FIRST_TIME_SEEN.into(),
            }))
        }
    }
}
