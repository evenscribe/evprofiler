#[derive(Debug, Clone, PartialEq)]
pub enum DebugInfoUploadReason {
    /// Debuginfo exists in debuginfod, therefore no upload is necessary.
    DebugInfoInDebugInfod,

    /// First time we see this Build ID, and it does not exist in debuginfod, therefore please upload!
    FirstTimeSeen,

    /// A previous upload was started but not finished and is now stale, so it can be retried.
    UploadStale,

    /// A previous upload is still in-progress and not stale yet (only stale uploads can be retried).
    UploadInProgress,

    /// Debuginfo already exists and is not marked as invalid, therefore no new upload is needed.
    DebugInfoAlreadyExists,

    /// Debuginfo already exists and is not marked as invalid, therefore wouldn't have accepted a new upload, 
    /// but accepting it because it's requested to be forced.
    DebugInfoAlreadyExistsButForced,

    /// Debuginfo already exists but is marked as invalid, therefore a new upload is needed. 
    /// Hash the debuginfo and initiate the upload.
    DebugInfoInvalid,

    /// Debuginfo already exists and is marked as invalid, but the proposed hash is the same as the 
    /// one already available, therefore the upload is not accepted as it would result in the same invalid debuginfos.
    DebugInfoEqual,

    /// Debuginfo already exists but is marked as invalid, therefore a new upload will be accepted.
    DebugInfoNotEqual,

    /// Debuginfo is available from debuginfod already and not marked as invalid, therefore no new upload is needed.
    DebugInfodSource,

    /// Debuginfo is available from debuginfod already but is marked as invalid, therefore a new upload is needed.
    DebugInfodInvalid,
}

impl DebugInfoUploadReason {
    /// Convert the enum variant to a descriptive string
    pub fn to_string(&self) -> String{
        let r = match self {
            Self::DebugInfoInDebugInfod => 
                "Debuginfo exists in debuginfod, therefore no upload is necessary.",
            Self::FirstTimeSeen => 
                "First time we see this Build ID, and it does not exist in debuginfod, therefore please upload!",
            Self::UploadStale => 
                "A previous upload was started but not finished and is now stale, so it can be retried.",
            Self::UploadInProgress => 
                "A previous upload is still in-progress and not stale yet (only stale uploads can be retried).",
            Self::DebugInfoAlreadyExists => 
                "Debuginfo already exists and is not marked as invalid, therefore no new upload is needed.",
            Self::DebugInfoAlreadyExistsButForced => 
                "Debuginfo already exists and is not marked as invalid, therefore wouldn't have accepted a new upload, but accepting it because it's requested to be forced.",
            Self::DebugInfoInvalid => 
                "Debuginfo already exists but is marked as invalid, therefore a new upload is needed. Hash the debuginfo and initiate the upload.",
            Self::DebugInfoEqual => 
                "Debuginfo already exists and is marked as invalid, but the proposed hash is the same as the one already available, therefore the upload is not accepted as it would result in the same invalid debuginfos.",
            Self::DebugInfoNotEqual => 
                "Debuginfo already exists but is marked as invalid, therefore a new upload will be accepted.",
            Self::DebugInfodSource => 
                "Debuginfo is available from debuginfod already and not marked as invalid, therefore no new upload is needed.",
            Self::DebugInfodInvalid => 
                "Debuginfo is available from debuginfod already but is marked as invalid, therefore a new upload is needed.",
        };
        r.to_string()
    }
}
