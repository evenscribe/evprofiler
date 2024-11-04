use crate::profilestorepb::agents_service_server::AgentsService;
use crate::profilestorepb::{AgentsRequest, AgentsResponse};
use std::result::Result;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct AgentStore {}

#[tonic::async_trait]
impl AgentsService for AgentStore {
    async fn agents(
        &self,
        request: Request<AgentsRequest>,
    ) -> Result<Response<AgentsResponse>, Status> {
        log::info!(
            "Received AgentsService::agents request \n body: {:?}",
            request
        );
        return Ok(Response::new(AgentsResponse { agents: vec![] }));
    }
}
