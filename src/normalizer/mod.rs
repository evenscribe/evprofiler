mod profile;
mod sample;
mod series;
mod utils;
mod write_raw;

use profile::NormalizedProfile;
pub use sample::NormalizedSample;
pub use series::Series;
pub use utils::write_raw_request_to_arrow_chunk;

pub const POSSIBLE_METADATA_LABELS: [&str; 20] = [
    "pid",
    "ppid",
    "arch",
    "systemd_unit",
    "node",
    "cgroup_name",
    "compiler",
    "stripped",
    "static",
    "comm",
    "executable",
    "kernel_release",
    "agent_revision",
    "buildid",
    "thread_id",
    "thread_name",
    "namespace",
    "pod",
    "container",
    "containerid",
];
