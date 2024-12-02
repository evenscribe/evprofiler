use super::{NormalizedProfile, Series};
use crate::pprofpb::Profile;
use crate::profilestorepb::WriteRawRequest;
use anyhow::bail;
use flate2::read::GzDecoder;
use prost::Message;
use std::collections::{HashMap, HashSet};
use std::io::Read;

#[derive(Debug)]
pub struct NormalizedWriteRawRequest {
    pub(crate) series: Vec<Series>,
    pub(crate) all_label_names: Vec<String>,
}

impl TryFrom<&WriteRawRequest> for NormalizedWriteRawRequest {
    type Error = anyhow::Error;

    fn try_from(request: &WriteRawRequest) -> anyhow::Result<Self> {
        let mut all_label_names: HashSet<String> = HashSet::new();
        let mut series: Vec<Series> = Vec::with_capacity(request.series.len());

        for raw_series in request.series.iter() {
            let mut ls: HashMap<String, String> = HashMap::new();
            let mut name: String = "".into();

            if let Some(label_set) = &raw_series.labels {
                for label in label_set.labels.iter() {
                    if label.name.eq("__name__") {
                        name = label.value.clone();
                        continue;
                    }

                    if ls.contains_key(&label.name) {
                        bail!("Duplicate label {} in series", label.name);
                    }

                    ls.insert(label.name.clone(), label.value.clone());
                    all_label_names.insert(label.name.clone());
                }
            }

            if name.is_empty() {
                bail!("Series must have a __name__ label");
            }

            let mut samples: Vec<Vec<NormalizedProfile>> =
                Vec::with_capacity(raw_series.samples.len());

            for sample in raw_series.samples.iter() {
                let mut decompressed = Vec::new();

                let mut decoder = GzDecoder::new(sample.raw_profile.as_slice());
                if decoder.header().is_none() {
                    if let Err(e) = decoder.read_to_end(&mut decompressed) {
                        bail!("Failed to decompress gzip: {}", e);
                    }
                }

                let p = Profile::decode(decompressed.as_slice())?;

                // let _ =
                super::utils::validate_pprof_profile(&p, sample.executable_info.as_slice())?;

                super::utils::label_names_from_profile(
                    &ls,
                    p.string_table.as_slice(),
                    p.sample.as_slice(),
                    &mut all_label_names,
                );

                let np: Vec<NormalizedProfile> =
                    super::utils::normalize_pprof(name.as_str(), &ls, &p)?;

                samples.push(np);
            }

            series.push(Series {
                labels: ls,
                samples,
            });
        }

        let all_label_names = Vec::from_iter(all_label_names);

        Ok(NormalizedWriteRawRequest {
            series,
            all_label_names,
        })
    }
}
