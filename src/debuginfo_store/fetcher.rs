use anyhow::bail;
use object_store::ObjectStore;

use super::DebugInfod;
use crate::debuginfopb::{debuginfo::Source, Debuginfo};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct DebuginfoFetcher {
    bucket: Arc<dyn ObjectStore>,
    debuginfod: DebugInfod,
}

impl DebuginfoFetcher {
    pub fn new(bucket: Arc<dyn ObjectStore>, debuginfod: DebugInfod) -> Self {
        Self { bucket, debuginfod }
    }

    pub async fn fetch_raw_elf(&self, dbginfo: &Debuginfo) -> anyhow::Result<Vec<u8>> {
        let source = dbginfo.source();
        match source {
            Source::Debuginfod => self.fetch_debuginfod(dbginfo).await,
            Source::Upload => self.fetch_bucket(dbginfo).await,
            _ => {
                bail!("Unknown source in Debuginfo");
            }
        }
    }

    async fn fetch_debuginfod(&self, dbginfo: &Debuginfo) -> anyhow::Result<Vec<u8>> {
        let rc = self
            .debuginfod
            .get(
                &self.debuginfod.upstream_servers[0],
                dbginfo.build_id.as_str(),
            )
            .await?;
        Ok(rc.to_vec())
    }

    async fn fetch_bucket(&self, dbginfo: &Debuginfo) -> anyhow::Result<Vec<u8>> {
        let path: &str = &dbginfo.upload.as_ref().unwrap().id;

        let rc = self
            .bucket
            .get(&object_store::path::Path::from(path))
            .await?;

        Ok(rc.bytes().await?.to_vec())
    }
}
