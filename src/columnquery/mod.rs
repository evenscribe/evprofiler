mod pprof_writer;
mod record_reader;
use crate::{dal::DataAccessLayer, pprofpb, profile};
use flate2::write::GzDecoder;
use pprof_writer::PprofWriter;
use prost::Message;
use std::{io::Write, sync::Arc};

pub struct ColumnQuery {
    dal: Arc<DataAccessLayer>,
}

pub enum ColumnQueryRequest {
    GeneratePprof,
}

pub enum ColumnQueryResponse {
    Pprof(Vec<u8>),
}

impl ColumnQuery {
    pub fn new(dal: &Arc<DataAccessLayer>) -> Self {
        Self {
            dal: Arc::clone(dal),
        }
    }

    pub async fn query(
        &self,
        query_type: ColumnQueryRequest,
        query_string: &str,
        timestamp: i64,
    ) -> anyhow::Result<ColumnQueryResponse> {
        let p: profile::Profile = self.dal.select_single(query_string, timestamp).await?;
        match query_type {
            ColumnQueryRequest::GeneratePprof => self.generate_pprof(p),
        }
    }

    pub fn generate_pprof(&self, profile: profile::Profile) -> anyhow::Result<ColumnQueryResponse> {
        let mut w = PprofWriter::new(profile.meta);
        for rec in profile.samples {
            w.write_record(rec)?;
        }
        let p: pprofpb::Profile = w.finish()?;
        let buf = serialize_pprof(&p)?;
        Ok(ColumnQueryResponse::Pprof(buf))
    }
}

fn serialize_pprof(pp: &pprofpb::Profile) -> anyhow::Result<Vec<u8>> {
    let data = pp.encode_to_vec();
    let mut gzipped = GzDecoder::new(Vec::new());
    gzipped.write_all(data.as_slice())?;
    Ok(gzipped.finish()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        debuginfo_store::{self, DebuginfoFetcher},
        storage, symbolizer,
    };
    use object_store::ObjectStore;

    #[tokio::test]
    async fn test_generate_pprof() {
        let metadata_store = debuginfo_store::MetadataStore::new();
        let debuginfod = debuginfo_store::DebugInfod::default();
        let debuginfod_bucket: Arc<dyn ObjectStore> = Arc::new(storage::new_memory_bucket());
        let symbolizer = Arc::new(symbolizer::Symbolizer::new(
            debuginfo_store::MetadataStore::with_store(metadata_store.store.clone()),
            DebuginfoFetcher::new(Arc::clone(&debuginfod_bucket), debuginfod.clone()),
        ));

        let dal = Arc::new(
            DataAccessLayer::try_new("evprofiler-data", 5000, &symbolizer)
                .await
                .unwrap(),
        );
        let column_query = ColumnQuery::new(&dal);
        let qs = "arch=aarch64|parca_agent_cpu:samples:count:cpu:nanoseconds";
        let x = column_query
            .query(ColumnQueryRequest::GeneratePprof, qs, 1734496813872)
            .await
            .unwrap();
    }
}
