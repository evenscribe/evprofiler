use chrono::TimeDelta;
use debuginfo_store::DebuginfoFetcher;
use debuginfopb::debuginfo_service_server::DebuginfoServiceServer;
use ingester::Ingester;
use object_store::{local, ObjectStore};
use profilestorepb::{
    agents_service_server::AgentsServiceServer,
    profile_store_service_server::ProfileStoreServiceServer,
};
use std::sync::Arc;
use tonic::{codec::CompressionEncoding, transport::Server};

mod agent_store;
mod columnquery;
mod debuginfo_store;
mod ingester;
mod normalizer;
mod profile;
mod profile_store;
mod storage;
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

    let metadata_store = debuginfo_store::MetadataStore::new();
    let debuginfod = debuginfo_store::DebugInfod::default();
    let debuginfod_bucket: Arc<dyn ObjectStore> = Arc::new(storage::new_memory_bucket());
    let stackrace_bucket: Arc<dyn ObjectStore> = Arc::new(
        match local::LocalFileSystem::new_with_prefix("evprofiler-data") {
            Ok(s) => s,
            Err(..) => {
                let _ = std::fs::create_dir("evprofiler-data");
                local::LocalFileSystem::new_with_prefix("evprofiler-data").unwrap()
            }
        },
    );
    let ingester = Arc::new(Ingester::new(10, Arc::clone(&stackrace_bucket)));
    let symbolizer = Arc::new(symbolizer::Symbolizer::new(
        debuginfo_store::MetadataStore::with_store(metadata_store.store.clone()),
        DebuginfoFetcher::new(Arc::clone(&debuginfod_bucket), debuginfod.clone()),
    ));

    log::info!("Starting Server");

    let addr = "[::1]:3333".parse().unwrap();

    log::info!("Attaching ProfileStoreService to the server");
    let profile_store_impl = profile_store::ProfileStore::new(symbolizer, ingester);

    log::info!("Attaching AgentsService to the server");
    let agent_store_impl = agent_store::AgentStore::default();

    log::info!("Attaching DebugInfo to the server");
    let debug_store_impl = debuginfo_store::DebuginfoStore {
        metadata: metadata_store,
        debuginfod,
        max_upload_duration: TimeDelta::new(60 * 15, 0).unwrap(),
        max_upload_size: 1000000000,
        bucket: Arc::clone(&debuginfod_bucket),
    };

    log::info!("Starting server at {}", addr);
    Server::builder()
        .add_service(
            ProfileStoreServiceServer::new(profile_store_impl)
                .accept_compressed(CompressionEncoding::Gzip)
                .max_decoding_message_size(1000000000)
                .max_encoding_message_size(1000000000),
        )
        .add_service(AgentsServiceServer::new(agent_store_impl))
        .add_service(
            DebuginfoServiceServer::new(debug_store_impl)
                .accept_compressed(CompressionEncoding::Gzip)
                .max_decoding_message_size(1000000000)
                .max_encoding_message_size(1000000000),
        )
        .serve(addr)
        .await?;

    Ok(())
}
