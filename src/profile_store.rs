use crate::profilestorepb::profile_store_service_server::ProfileStoreService;
use crate::profilestorepb::{WriteRawRequest, WriteRawResponse, WriteRequest, WriteResponse};
use std::{pin::Pin, result::Result};
use tokio_stream::Stream;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
struct ProfileStore {}

#[tonic::async_trait]
impl ProfileStoreService for ProfileStore {
    /// WriteRaw accepts a raw set of bytes of a pprof file
    async fn write_raw(
        &self,
        request: Request<WriteRawRequest>,
    ) -> Result<Response<WriteRawResponse>, Status> {
        todo!();
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
        request: Request<tonic::Streaming<WriteRequest>>,
    ) -> Result<Response<Self::WriteStream>, Status> {
        todo!();
    }
}
