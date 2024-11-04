use super::profile::Meta;
use crate::profilestorepb::WriteRawRequest;
use std::collections::{HashMap, HashSet};
use std::result::Result;
use tonic::Status;

#[derive(Debug)]
pub struct NormalizedProfile {
    pub(crate) samples: Vec<NormalizedSample>,
    pub(crate) meta: Meta,
}

#[derive(Debug)]
pub struct NormalizedSample {
    pub(crate) locations: Vec<Vec<u8>>,
    pub(crate) value: i64,
    pub(crate) diff_value: i64,
    pub(crate) label: HashMap<String, String>,
    pub(crate) num_label: HashMap<String, i64>,
}

#[derive(Debug)]
pub struct Series {
    pub(crate) labels: HashMap<String, String>,
    pub(crate) samples: Vec<Vec<NormalizedProfile>>,
}

#[derive(Debug)]
pub struct NormalizedWriteRawRequest {
    series: Vec<Series>,
    all_label_names: Vec<String>,
}

impl TryFrom<&WriteRawRequest> for NormalizedWriteRawRequest {
    type Error = Status;

    fn try_from(request: &WriteRawRequest) -> Result<Self, Self::Error> {
        let mut all_label_names: HashSet<String> = HashSet::new();
        for raw_series in request.series.iter() {
            let mut ls: HashMap<String, String> = HashMap::new();
            let mut name: String = "".into();

            for label_set in raw_series.labels.iter() {
                for label in label_set.labels.iter() {
                    if label.name == "__name__" {
                        name = label.value.clone();
                    }

                    if ls.contains_key(&label.name) {
                        return Err(Status::invalid_argument(format!(
                            "Duplicate label {} in series",
                            label.name
                        )));
                    }

                    ls.insert(label.name.clone(), label.value.clone());
                    all_label_names.insert(label.name.clone());
                }
            }

            if name.is_empty() {
                return Err(Status::invalid_argument(
                    "Series must have a __name__ label",
                ));
            }
        }

        Ok(NormalizedWriteRawRequest {
            series: vec![],
            all_label_names: vec![],
        })
    }
}
