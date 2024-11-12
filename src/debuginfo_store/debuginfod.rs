use crate::debuginfopb::Debuginfo;
use std::{collections::HashMap, time::Duration};
use tonic::Status;
use ureq;
use url::Url;

pub struct DebugInfod {
    upstream_server: Url,
    bucket: HashMap<String, Vec<u8>>,
    client: ureq::Agent,
}

impl DebugInfod {
    pub fn default() -> Self {
        let url = Url::parse("https://debuginfod.elfutils.org/").unwrap();

        Self {
            upstream_server: url,
            bucket: HashMap::new(),
            client: ureq::AgentBuilder::new()
                .timeout_read(Duration::from_secs(5))
                .timeout_write(Duration::from_secs(5))
                .redirects(2)
                .build(),
        }
    }

    pub fn exists(&mut self, build_id: &str) -> bool {
        match self.get(build_id) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn get(&mut self, build_id: &str) -> Result<&[u8], Status> {
        self.debuginfo_request(build_id)
    }

    fn debuginfo_request(&mut self, build_id: &str) -> Result<&[u8], Status> {
        let url = self.upstream_server.clone();
        url.join(format!("buildid/{}/debuginfo", build_id).as_str())
            .unwrap();
        self.request(url)
    }

    fn request(&mut self, url: Url) -> Result<&[u8], Status> {
        if !self.bucket.contains_key(url.as_str()) {
            let response = match self.client.get(url.as_str()).call() {
                Ok(response) => response,
                Err(err) => {
                    return Err(Status::internal(format!(
                        "Failed to fetch debuginfo: {}",
                        err
                    )));
                }
            };

            if response.status() == 200 {
                let mut content = Vec::new();
                response.into_reader().read_to_end(&mut content)?;
                self.bucket.insert(url.to_string(), content);
            } else {
                return Err(Status::internal(format!(
                    "Failed to fetch debuginfo: {}",
                    response.status()
                )));
            }
        }

        Ok(self.bucket.get(url.as_str()).unwrap())
    }
}
