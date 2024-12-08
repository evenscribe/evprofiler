use crate::profilestorepb::profile_store_service_server::ProfileStoreService;
use crate::profilestorepb::{WriteRawRequest, WriteRawResponse, WriteRequest, WriteResponse};
use crate::{normalizer, symbolizer};
use anyhow::bail;
use std::sync::Arc;
use std::{pin::Pin, result::Result};
use tokio_stream::Stream;
use tonic::{Request, Response, Status, Streaming};

#[derive(Debug)]
pub struct ProfileStore {
    symbolizer: Arc<symbolizer::Symbolizer>,
}

#[tonic::async_trait]
impl ProfileStoreService for ProfileStore {
    /// WriteRaw accepts a raw set of bytes of a pprof file
    async fn write_raw(
        &self,
        request: Request<WriteRawRequest>,
    ) -> anyhow::Result<Response<WriteRawResponse>, Status> {
        let _ = match self.write_series(&request.into_inner()).await {
            Ok(_) => (),
            Err(e) => return Err(Status::internal(e.to_string())),
        };
        return Ok(Response::new(WriteRawResponse {}));
    }
    /// Server streaming response type for the Write method.
    type WriteStream =
        Pin<Box<dyn Stream<Item = Result<WriteResponse, Status>> + std::marker::Send + 'static>>;

    /// Write accepts profiling data encoded as an arrow record. It's a
    /// bi-directional streaming RPC, because the first message can contain only
    /// samples without the stacktraces, and only reference stacktrace IDs. The
    /// backend can then request the full stacktrace from the client should it not
    /// know the stacktrace yet.
    async fn write(
        &self,
        request: Request<Streaming<WriteRequest>>,
    ) -> anyhow::Result<Response<Self::WriteStream>, Status> {
        let mut stream = request.into_inner();

        log::info!("Received ProfileStoreService::write request",);

        let output = async_stream::try_stream! {
            while let Some(request) = stream.message().await? {
            log::info!(
                "Received ProfileStoreService::write request \n Record: {:?} ",
                request.record,
            );
            yield WriteResponse {record: vec![]};
            }
        };

        Ok(Response::new(Box::pin(output)))
    }
}

impl ProfileStore {
    pub fn new(symbolizer: Arc<symbolizer::Symbolizer>) -> Self {
        Self {
            symbolizer: Arc::clone(&symbolizer),
        }
    }

    pub async fn write_series(&self, request: &WriteRawRequest) -> anyhow::Result<()> {
        let (chunk, schema) = match normalizer::write_raw_request_to_arrow_chunk(request).await {
            Ok(record) => record,
            Err(e) => {
                bail!(
                    "Failed to normalize WriteRawRequest to Arrow Record, details: {}",
                    e
                );
            }
        };

        if chunk.is_empty() {
            return Ok(());
        }

        Ok(())
    }
}
