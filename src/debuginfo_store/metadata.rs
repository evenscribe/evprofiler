use crate::debuginfopb::{self, Debuginfo, DebuginfoType};
use std::collections::HashMap;

use self::debuginfopb::debuginfo::Source;

#[derive(Debug, Default)]
pub struct MetadataStore {
    store: HashMap<String, Debuginfo>,
}

impl MetadataStore {
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }

    pub fn fetch(&self, build_id: &str, req_type: &DebuginfoType) -> Option<&Debuginfo> {
        let path = Self::get_object_path(build_id, req_type);
        self.store.get(&path)
    }

    fn get_object_path(build_id: &str, req_type: &DebuginfoType) -> String {
        match req_type {
            DebuginfoType::Executable => format!("{}/executable.metadata", build_id),
            DebuginfoType::Sources => format!("{}/sources.metadata", build_id),
            _ => format!("{}/metadata", build_id),
        }
    }

    pub fn mark(&mut self, build_id: &str, req_type: &DebuginfoType) {
        let path = Self::get_object_path(build_id, req_type);
        self.store.insert(
            path,
            Debuginfo {
                build_id: build_id.to_string(),
                r#type: req_type.clone().into(),
                source: Source::Debuginfod.into(),
                upload: None,
                quality: None,
                debuginfod_servers: vec![],
            },
        );
    }
}
