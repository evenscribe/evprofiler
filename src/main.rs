use profilestorepb::{
    agents_service_server::AgentsServiceServer,
    profile_store_service_server::ProfileStoreServiceServer,
};
use tonic::transport::Server;

mod agent_store;
mod normalizer;
mod profile;
mod profile_store;

pub(crate) mod profilestorepb {
    tonic::include_proto!("parca.profilestore.v1alpha1");
}

pub(crate) mod metapb {
    tonic::include_proto!("parca.metastore.v1alpha1");
}

pub(crate) mod pprofpb {
    tonic::include_proto!("perftools.profiles");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    log::info!("Starting Server");

    let addr = "[::1]:3333".parse().unwrap();

    log::info!("Attaching ProfileStoreService to the server");
    let profile_store_impl = profile_store::ProfileStore::default();

    log::info!("Attaching AgentsService to the server");
    let agent_store_impl = agent_store::AgentStore::default();

    log::info!("Starting server at {}", addr);
    Server::builder()
        .add_service(ProfileStoreServiceServer::new(profile_store_impl))
        .add_service(AgentsServiceServer::new(agent_store_impl))
        .serve(addr)
        .await?;

    Ok(())
}
