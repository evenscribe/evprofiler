use crate::profile::LocationLine;

use super::normalize::NormalizedAddress;
use moka::sync::Cache;

#[derive(Debug, Clone)]
pub struct SymbolizerCache {
    pub(crate) c: Cache<Vec<u8>, Vec<Vec<u8>>>,
}

impl Default for SymbolizerCache {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl SymbolizerCache {
    pub fn new(cap: u64) -> Self {
        let c = Cache::new(cap);
        Self { c }
    }

    pub fn get(
        &self,
        build_id: &str,
        addr: &NormalizedAddress,
    ) -> anyhow::Result<Option<Vec<LocationLine>>> {
        let key = Self::build_cache_key(build_id, addr);
        let ll = match self.c.get(&key) {
            Some(ll) => ll,
            None => return Ok(None),
        };
        Ok(Some(Self::decode(&ll)?))
    }

    pub fn set(
        &self,
        build_id: &str,
        addr: &NormalizedAddress,
        ll: Vec<LocationLine>,
    ) -> anyhow::Result<()> {
        let key = Self::build_cache_key(build_id, addr);
        let mut encoded = vec![];

        for line in ll.iter() {
            encoded.push(line.encode()?);
        }

        Ok(self.c.insert(key, encoded))
    }

    fn build_cache_key(build_id: &str, addr: &NormalizedAddress) -> Vec<u8> {
        format!("{}/0x{}", build_id, addr.0).as_bytes().to_vec()
    }

    fn decode(ll: &[Vec<u8>]) -> anyhow::Result<Vec<LocationLine>> {
        let mut res: Vec<LocationLine> = vec![];

        for line in ll.iter() {
            res.push(LocationLine::decode(line)?);
        }

        Ok(res)
    }
}
