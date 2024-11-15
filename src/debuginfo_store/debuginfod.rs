use std::{collections::HashMap, time::Duration};
use tonic::Status;
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
        self.get(build_id).is_ok()
    }

    pub fn get(&mut self, build_id: &str) -> Result<&[u8], Status> {
        self.debuginfo_request(build_id)
    }

    fn debuginfo_request(&mut self, build_id: &str) -> Result<&[u8], Status> {
        let url = self
            .upstream_server
            .join(format!("buildid/{}/debuginfo", build_id).as_str())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debuginfod_get() {
        let mut debuginfod = DebugInfod::default();

        // testing for linux's clear exec build id
        let debug_ = debuginfod
            .get("252f7dc22ca9d935e8334f04a0232f35359b5880")
            .unwrap();

        assert_eq!(debug_.is_empty(), false);
    }

    #[test]
    fn test_debuginfod_exists() {
        let mut debuginfod = DebugInfod::default();
        // testing for a random buildid
        assert_eq!(debuginfod.exists("123"), false);

        // testing for linux's clear exec build id
        assert_eq!(
            debuginfod.exists("252f7dc22ca9d935e8334f04a0232f35359b5880"),
            true,
        );
    }
}
