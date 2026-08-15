use std::fmt;

#[derive(Debug)]
pub enum VdriftError {
    /// Exit 1 — version drift / verification failure
    Drift(String),
    /// Exit 2 — invalid configuration
    Config(String),
    /// Exit 3 — unsupported project
    Unsupported(String),
    /// Exit 4 — Git failure
    Git(String),
    /// Exit 5 — user cancelled
    Cancelled(String),
    /// Exit 6 — unsafe working tree
    UnsafeTree(String),
    /// Exit 7 — invalid version
    InvalidVersion(String),
    /// Exit 8 — adapter failure
    Adapter(String),
    /// Exit 10 — internal error
    Internal(String),
}

impl VdriftError {
    pub fn exit_code(&self) -> i32 {
        match self {
            VdriftError::Drift(_) => 1,
            VdriftError::Config(_) => 2,
            VdriftError::Unsupported(_) => 3,
            VdriftError::Git(_) => 4,
            VdriftError::Cancelled(_) => 5,
            VdriftError::UnsafeTree(_) => 6,
            VdriftError::InvalidVersion(_) => 7,
            VdriftError::Adapter(_) => 8,
            VdriftError::Internal(_) => 10,
        }
    }

    /// Stable machine-readable code string for JSON error output.
    pub fn code(&self) -> &'static str {
        match self {
            VdriftError::Drift(_) => "VERSION_DRIFT",
            VdriftError::Config(_) => "INVALID_CONFIGURATION",
            VdriftError::Unsupported(_) => "UNSUPPORTED_PROJECT",
            VdriftError::Git(_) => "GIT_FAILURE",
            VdriftError::Cancelled(_) => "USER_CANCELLED",
            VdriftError::UnsafeTree(_) => "UNSAFE_WORKING_TREE",
            VdriftError::InvalidVersion(_) => "INVALID_VERSION",
            VdriftError::Adapter(_) => "ADAPTER_FAILURE",
            VdriftError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn multiple_version_sources() -> Self {
        VdriftError::Config(
            "Multiple canonical version sources detected. \
             Configure the project version source in .vdrift.toml."
                .into(),
        )
    }
}

impl fmt::Display for VdriftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VdriftError::Drift(m)
            | VdriftError::Config(m)
            | VdriftError::Unsupported(m)
            | VdriftError::Git(m)
            | VdriftError::Cancelled(m)
            | VdriftError::UnsafeTree(m)
            | VdriftError::InvalidVersion(m)
            | VdriftError::Adapter(m)
            | VdriftError::Internal(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for VdriftError {}

pub type Result<T> = std::result::Result<T, VdriftError>;
