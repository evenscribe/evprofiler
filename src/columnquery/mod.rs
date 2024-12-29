mod pprof_writer;

use pprof_writer::PprofWriter;

use crate::{dal::DataAccessLayer, profile};
use std::sync::Arc;

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
    pub fn new(dal: Arc<DataAccessLayer>) -> Self {
        Self { dal }
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
        w.finish()
    }
}

//pub fn generate_flat_pprof() -> pprofpb::Profile {}
