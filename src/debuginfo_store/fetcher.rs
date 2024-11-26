use super::DebugInfod;
use crate::debuginfopb::{debuginfo::Source, Debuginfo};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
pub struct DebuginfoFetcher {
    bucket: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    debuginfod: Arc<Mutex<DebugInfod>>,
}

impl DebuginfoFetcher {
    pub fn new(
        bucket: Arc<Mutex<HashMap<String, Vec<u8>>>>,
        debuginfod: Arc<Mutex<DebugInfod>>,
    ) -> Self {
        Self { bucket, debuginfod }
    }

    pub fn fetch_raw_elf(&self, dbginfo: &Debuginfo) -> Result<Vec<u8>, tonic::Status> {
        let source = dbginfo.source();
        match source {
            Source::Debuginfod => self.fetch_debuginfod(dbginfo),
            Source::Upload => self.fetch_bucket(dbginfo),
            _ => Err(tonic::Status::internal("Unknown source in Debuginfo")),
        }
    }

    fn fetch_debuginfod(&self, dbginfo: &Debuginfo) -> Result<Vec<u8>, tonic::Status> {
        let mut debuginfod = match self.debuginfod.lock() {
            Ok(debuginfod) => debuginfod,
            Err(_) => return Err(tonic::Status::internal("Failed to lock DebugInfod")),
        };

        let servers = debuginfod.upstream_servers.clone();
        let rc = debuginfod.get(&servers[0], dbginfo.build_id.as_str())?;
        return Ok(rc.to_vec());
    }

    fn fetch_bucket(&self, dbginfo: &Debuginfo) -> Result<Vec<u8>, tonic::Status> {
        let bucket = match self.bucket.lock() {
            Ok(bucket) => bucket,
            Err(_) => return Err(tonic::Status::internal("Failed to lock bucket")),
        };
        let path = &dbginfo.upload.as_ref().unwrap().id;
        if let Some(rc) = bucket.get(path) {
            return Ok(rc.clone());
        }
        Err(tonic::Status::internal("No data found in bucket"))
    }
}
