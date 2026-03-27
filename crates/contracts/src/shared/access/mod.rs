//! Access control types shared between backend and frontend.
//!
//! All types use `'static` lifetimes and `Copy` for zero-cost compile-time constants.

use serde::{Deserialize, Serialize};

// ============================================================================
// Core access types
// ============================================================================

/// Access mode for a scope.
/// Used in the role × scope → mode matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccessMode {
    Read,
    All,
}

impl AccessMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::All => "all",
        }
    }

    /// Returns true if `self` satisfies the `required` level.
    pub fn satisfies(&self, required: AccessMode) -> bool {
        match required {
            AccessMode::Read => matches!(self, AccessMode::Read | AccessMode::All),
            AccessMode::All => matches!(self, AccessMode::All),
        }
    }
}

// ============================================================================
// Metadata types (compile-time constants, Copy)
// ============================================================================

/// A single logical operation on an entity (list, get, upsert, delete).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeOperation {
    /// Operation identifier: "list", "get", "upsert", "delete"
    pub id: &'static str,
    /// Minimum access mode required for this operation
    pub required_mode: AccessMode,
}

/// Access metadata for an entity, embedded in `EntityMetadataInfo`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityAccessMeta {
    /// Stable scope identifier — matches the aggregate folder name (e.g. "a001_connection_1c")
    pub scope_id: &'static str,
    /// Logical operations and their required access modes
    pub operations: &'static [ScopeOperation],
}

// ============================================================================
// Runtime types (serializable, used in API responses)
// ============================================================================

/// One entry in the user's effective scope map, returned by /api/system/auth/me.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAccess {
    pub scope_id: String,
    /// "read" or "all"
    pub mode: String,
}
