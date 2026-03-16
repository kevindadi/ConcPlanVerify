use std::fmt;

/// Error codes and messages produced during CIR → CVN translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranslateError {
    // ── T0xx: Invalid CIR input ──

    /// T001 — The program's `entry` field names a function that does not exist.
    MissingEntry(String),
    /// T002 — The entry function has an empty body.
    EmptyEntryBody(String),
    /// T003 — A spawn/join/call references a function that is neither defined nor summarized.
    UnknownFunction(String),

    // ── T1xx: Resource translation errors ──

    /// T101 — Unrecognized resource type string.
    UnknownResourceType(String),
    /// T102 — Condvar `wait` references a lock that does not exist in resources.
    CondvarLockNotFound(String),
    /// T103 — Condvar `wait` references a lock that is not a Mutex.
    CondvarLockNotMutex(String),

    // ── T2xx: Control-flow translation errors ──

    /// T201 — A transfer target sid does not exist in the function body.
    InvalidTarget {
        sid: String,
        fn_name: String,
    },
    /// T202 — Branch condition string cannot be parsed.
    InvalidBranchCondition(String),
    /// T203 — Switch variable is not an Enum type.
    SwitchNotEnum(String),

    // ── T3xx: Consistency errors ──

    /// T301 — Cannot determine whether a RwLock drop releases a read-lock or write-lock.
    AmbiguousRwLockDrop {
        fn_name: String,
        sid: String,
    },
    /// T302 — A condvar notify/notify_all has no corresponding wait-sites.
    NoWaitSites(String),

    // ── Builder / internal errors ──

    /// Wrapper for errors produced by [`CvnNetBuilder::build`].
    BuilderError(String),
}

impl fmt::Display for TranslateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEntry(name) => {
                write!(f, "T001: program entry function '{name}' not found")
            }
            Self::EmptyEntryBody(name) => {
                write!(f, "T002: entry function '{name}' has empty body")
            }
            Self::UnknownFunction(name) => {
                write!(f, "T003: spawn/join/call references unknown function '{name}'")
            }
            Self::UnknownResourceType(ty) => {
                write!(f, "T101: unknown resource type '{ty}'")
            }
            Self::CondvarLockNotFound(lock) => {
                write!(f, "T102: condvar wait references non-existent lock '{lock}'")
            }
            Self::CondvarLockNotMutex(lock) => {
                write!(f, "T103: condvar wait lock '{lock}' is not a Mutex")
            }
            Self::InvalidTarget { sid, fn_name } => {
                write!(f, "T201: transfer target sid '{sid}' not found in function '{fn_name}'")
            }
            Self::InvalidBranchCondition(cond) => {
                write!(f, "T202: invalid branch condition '{cond}'")
            }
            Self::SwitchNotEnum(var) => {
                write!(f, "T203: switch variable '{var}' is not Enum type")
            }
            Self::AmbiguousRwLockDrop { fn_name, sid } => {
                write!(
                    f,
                    "T301: cannot determine lock kind for RwLock drop at {fn_name}:{sid}"
                )
            }
            Self::NoWaitSites(cv) => {
                write!(f, "T302: condvar notify for '{cv}' has no wait-sites")
            }
            Self::BuilderError(msg) => {
                write!(f, "CVN builder error: {msg}")
            }
        }
    }
}

impl std::error::Error for TranslateError {}
