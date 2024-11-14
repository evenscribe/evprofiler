mod debuginfod;
mod metadata;

use crate::debuginfopb::debuginfo_service_server::DebuginfoService;
use crate::debuginfopb::{
    BuildIdType, InitiateUploadRequest, InitiateUploadResponse, MarkUploadFinishedRequest,
    MarkUploadFinishedResponse, ShouldInitiateUploadRequest, ShouldInitiateUploadResponse,
    UploadRequest, UploadResponse,
};
use debuginfod::DebugInfod;
use metadata::MetadataStore;
use tonic::{async_trait, Response, Status};

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
    metadata: MetadataStore,
    debuginfod: DebugInfod,
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

        match self.metadata.fetch(&build_id, req_type) {
            Some(debuginfo) => {}
            None => {
                // First time we see this Build ID.
                let build_id_type = request.build_id_type();
                if build_id_type == BuildIdType::Gnu
                    || build_id_type == BuildIdType::UnknownUnspecified
                {
                    // if self.debuginfod.exists(&build_id) {}
                    Err(Status::invalid_argument(REASON_FIRST_TIME_SEEN))?;
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
