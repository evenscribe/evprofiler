use anyhow::{bail, Context};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tonic::Status;
use url::Url;

#[derive(Debug)]
pub struct DebugInfod {
    pub upstream_servers: Vec<Url>,
    bucket: HashMap<String, Vec<u8>>,
    client: ureq::Agent,
}

impl Default for DebugInfod {
    fn default() -> Self {
        let url = Url::parse("https://debuginfod.elfutils.org/").unwrap();

        Self {
            upstream_servers: vec![url],
            bucket: HashMap::new(),
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(5))
                .timeout_write(Duration::from_secs(5))
                .redirects(2)
                .build(),
        }
    }
}

impl DebugInfod {
    pub fn exists(&mut self, build_id: &str) -> Vec<String> {
        let mut available_servers = vec![];

        let vec = self.upstream_servers.clone();
        for server in vec {
            if self.get(&server, build_id).is_ok() {
                available_servers.push(server.to_string());
            }
        }
        available_servers
    }

    pub fn get(&mut self, upstream_server: &Url, build_id: &str) -> anyhow::Result<&[u8]> {
        self.debuginfo_request(upstream_server, build_id)
    }

    fn debuginfo_request(
        &mut self,
        upstream_server: &Url,
        build_id: &str,
    ) -> anyhow::Result<&[u8]> {
        let url = upstream_server.join(format!("buildid/{}/debuginfo", build_id).as_str())?;

        self.request(url)
    }

    fn request(&mut self, url: Url) -> anyhow::Result<&[u8]> {
        if !self.bucket.contains_key(url.as_str()) {
            let response =
                self.client.get(url.as_str()).call().map_err(|err| {
                    Status::internal(format!("Failed to fetch debuginfo: {}", err))
                })?;

            if response.status() == 200 {
                let mut content = Vec::new();
                response
                    .into_reader()
                    .read_to_end(&mut content)
                    .with_context(|| {
                        format!("Failed to read response from the debuginfod server")
                    })?;

                self.bucket.insert(url.to_string(), content);
            } else {
                bail!("Failed to fetch debuginfo: {}", response.status());
            }
        }

        Ok(self.bucket.get(url.as_str()).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debuginfod_get() {
        let mut debuginfod = DebugInfod::default();
        let srv = debuginfod.upstream_servers[0].clone();

        // testing for linux's clear exec build id
        let debug_ = debuginfod
            .get(&srv, "252f7dc22ca9d935e8334f04a0232f35359b5880")
            .unwrap();

        assert_eq!(debug_.is_empty(), false);
    }

    #[test]
    fn test_debuginfod_exists() {
        let mut debuginfod = DebugInfod::default();
        // testing for a random buildid
        assert_eq!(debuginfod.exists("123").is_empty(), true);

        // testing for linux's clear exec build id
        assert_eq!(
            debuginfod
                .exists("252f7dc22ca9d935e8334f04a0232f35359b5880")
                .is_empty(),
            false,
        );
    }
}
