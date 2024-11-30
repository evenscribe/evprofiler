use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{self, Duration},
};

use chrono::TimeDelta;
use debuginfo_store::DebuginfoFetcher;
use debuginfopb::debuginfo_service_server::DebuginfoServiceServer;
use profilestorepb::{
    agents_service_server::AgentsServiceServer,
    profile_store_service_server::ProfileStoreServiceServer,
};
use tonic::transport::Server;

mod agent_store;
mod debuginfo_store;
mod normalizer;
mod profile;
mod profile_store;
mod symbolizer;
mod symbols;

pub(crate) mod profilestorepb {
    tonic::include_proto!("parca.profilestore.v1alpha1");
}

pub(crate) mod metapb {
    tonic::include_proto!("parca.metastore.v1alpha1");
}

pub(crate) mod pprofpb {
    tonic::include_proto!("perftools.profiles");
}

pub(crate) mod debuginfopb {
    tonic::include_proto!("parca.debuginfo.v1alpha1");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    colog::init();

    let metadata_store = Arc::new(Mutex::new(debuginfo_store::MetadataStore::new()));
    let debuginfod = Arc::new(Mutex::new(debuginfo_store::DebugInfod::default()));
    let bucket: Arc<Mutex<HashMap<String, Vec<u8>>>> = Arc::new(Mutex::from(HashMap::new()));
    let symbolizer = Arc::new(symbolizer::Symbolizer::new(
        Arc::clone(&metadata_store),
        DebuginfoFetcher::new(Arc::clone(&bucket), Arc::clone(&debuginfod)),
    ));

    log::info!("Starting Server");

    let addr = "[::1]:3333".parse().unwrap();

    log::info!("Attaching ProfileStoreService to the server");
    let profile_store_impl = profile_store::ProfileStore::new(Arc::clone(&symbolizer));

    log::info!("Attaching AgentsService to the server");
    let agent_store_impl = agent_store::AgentStore::default();

    log::info!("Attaching DebugInfo to the server");
    let debug_store_impl = debuginfo_store::DebuginfoStore {
        metadata: Arc::clone(&metadata_store),
        debuginfod: Arc::clone(&debuginfod),
        max_upload_duration: TimeDelta::new(60 * 15, 0).unwrap(),
        max_upload_size: 1000000000,
        bucket: Arc::clone(&bucket),
    };

    log::info!("Starting server at {}", addr);
    Server::builder()
        .add_service(ProfileStoreServiceServer::new(profile_store_impl))
        .add_service(AgentsServiceServer::new(agent_store_impl))
        .add_service(DebuginfoServiceServer::new(debug_store_impl))
        .serve(addr)
        .await?;

    Ok(())
}
