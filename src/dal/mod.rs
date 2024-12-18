use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use datafusion::{
    catalog::TableProvider,
    datasource::{
        file_format::parquet::ParquetFormat,
        listing::{ListingOptions, ListingTable, ListingTableConfig, ListingTableUrl},
    },
    prelude::SessionContext,
};

struct CachedProvider {
    provider: Arc<dyn TableProvider>,
    created_at: Instant,
}

impl CachedProvider {
    fn new(provider: Arc<dyn TableProvider>) -> Self {
        Self {
            provider,
            created_at: Instant::now(),
        }
    }
}

pub struct DataAccessLayer {
    path_prefix: String,
    max_cache_stale_duration: Duration,
    config: ListingTableConfig,
    cached_provider: Mutex<CachedProvider>,
}

impl DataAccessLayer {
    pub async fn try_new(path: &str, cache_stale_duration: u64) -> anyhow::Result<Self> {
        let ctx = SessionContext::new();
        let session_state = ctx.state();
        let table_path = ListingTableUrl::parse(path)?;

        let file_format = ParquetFormat::new();
        let listing_options =
            ListingOptions::new(Arc::new(file_format)).with_file_extension(".parquet");

        let resolved_schema = listing_options
            .infer_schema(&session_state, &table_path)
            .await?;

        let config = ListingTableConfig::new(table_path)
            .with_listing_options(listing_options)
            .with_schema(resolved_schema);

        let provider = Arc::new(ListingTable::try_new(config.clone())?);

        Ok(Self {
            max_cache_stale_duration: Duration::new(cache_stale_duration, 0),
            path_prefix: path.to_string(),
            cached_provider: Mutex::new(CachedProvider::new(provider)),
            config,
        })
    }

    async fn get_provider(&self) -> anyhow::Result<Arc<dyn TableProvider>> {
        let mut cp = self.cached_provider.lock().unwrap();
        if cp.created_at.elapsed() < self.max_cache_stale_duration {
            return Ok(Arc::clone(&cp.provider));
        }

        let cp_ = self.create_cached_provider()?;
        let p = Arc::clone(&cp.provider);
        *cp = cp_;

        return Ok(p);
    }

    fn create_cached_provider(&self) -> anyhow::Result<CachedProvider> {
        let p = ListingTable::try_new(self.config.clone())?;
        Ok(CachedProvider::new(Arc::new(p)))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_provider_works() {
        let dal = DataAccessLayer::try_new("evprofiler-data", 5000)
            .await
            .unwrap();

        let ctx = SessionContext::new();

        let df = ctx.read_table(dal.get_provider().await.unwrap()).unwrap();
        let df = df
            .select_columns(&["labels.compiler", "labels.node"])
            .unwrap();
        let _ = df.show().await;
    }
}
