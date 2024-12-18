use anyhow::{bail, Context};
use object_store::ObjectStore;
use std::{sync::Arc, time::Duration};
use tonic::Status;
use url::Url;

#[derive(Debug)]
pub struct DebugInfod {
    pub upstream_servers: Vec<Url>,
    bucket: Arc<dyn ObjectStore>,
    client: ureq::Agent,
}

impl Clone for DebugInfod {
    fn clone(&self) -> Self {
        Self {
            upstream_servers: self.upstream_servers.clone(),
            bucket: Arc::clone(&self.bucket),
            client: self.client.clone(),
        }
    }
}

impl Default for DebugInfod {
    fn default() -> Self {
        let url = Url::parse("https://debuginfod.elfutils.org/").unwrap();

        Self {
            upstream_servers: vec![url],
            bucket: Arc::new(crate::storage::new_memory_bucket()),
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(5))
                .timeout_write(Duration::from_secs(5))
                .redirects(2)
                .build(),
        }
    }
}

impl DebugInfod {
    pub async fn exists(&self, build_id: &str) -> Vec<String> {
        let mut available_servers = vec![];

        let vec = self.upstream_servers.clone();
        for server in vec {
            if self.get(&server, build_id).await.is_ok() {
                available_servers.push(server.to_string());
            }
        }
        available_servers
    }

    pub async fn get(&self, upstream_server: &Url, build_id: &str) -> anyhow::Result<Vec<u8>> {
        self.debuginfo_request(upstream_server, build_id).await
    }

    async fn debuginfo_request(
        &self,
        upstream_server: &Url,
        build_id: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let url = upstream_server.join(format!("buildid/{}/debuginfo", build_id).as_str())?;

        self.request(url).await
    }

    async fn request(&self, url: Url) -> anyhow::Result<Vec<u8>> {
        let path = object_store::path::Path::from(url.as_str());
        let res = self.bucket.get(&path).await?.bytes().await?;
        if res.is_empty() {
            let response =
                self.client.get(url.as_str()).call().map_err(|err| {
                    Status::internal(format!("Failed to fetch debuginfo: {}", err))
                })?;

            if response.status() == 200 {
                let mut content = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut content)
                    .with_context(|| "Failed to read response from the debuginfod server")?;

                std::mem::drop(self.bucket.put(&path, content.clone().into()));
                Ok(content)
            } else {
                bail!("Failed to fetch debuginfo: {}", response.status());
            }
        } else {
            Ok(res.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_debuginfod_get() {
        let debuginfod = DebugInfod::default();
        let srv = debuginfod.upstream_servers[0].clone();

        // testing for linux's clear exec build id
        let debug_ = debuginfod
            .get(&srv, "252f7dc22ca9d935e8334f04a0232f35359b5880")
            .await
            .unwrap();

        assert_eq!(debug_.is_empty(), false);
    }

    #[tokio::test]
    async fn test_debuginfod_exists() {
        let debuginfod = DebugInfod::default();
        // testing for a random buildid
        assert_eq!(debuginfod.exists("123").await.is_empty(), true);

        // testing for linux's clear exec build id
        assert_eq!(
            debuginfod
                .exists("252f7dc22ca9d935e8334f04a0232f35359b5880")
                .await
                .is_empty(),
            false,
        );
    }
}
