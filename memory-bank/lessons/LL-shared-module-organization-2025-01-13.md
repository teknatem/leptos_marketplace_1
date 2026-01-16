---
title: Lesson - Shared Module Organization and Usage
date: 2025-01-13
tags: [lesson, architecture, shared-utilities, code-organization]
project: leptos_marketplace
---

# Lesson: Shared Module Organization and Usage

## Context

During code duplication refactoring, discovered that the `shared` module structure works well but requires consistent usage patterns. Only 1 out of 18 modules correctly imported `api_base()` from shared utilities.

## Key Learnings

### 1. Shared Module is Underutilized

**Observation:**

- `shared/api_utils.rs` contained public `api_base()` function
- 17 modules had duplicate implementations
- Only `a012_wb_sales/ui/details/model.rs` used the shared version

**Root Cause:**

- Developers aren't checking shared module before implementing utilities
- Lack of visibility into what's available in shared
- Copy-paste pattern propagates duplicates

**Lesson:**
Always check `crates/frontend/src/shared/` before implementing utility functions. If it doesn't exist there, add it there first.

### 2. Module Structure is Sound

**Current Structure:**

```
shared/
├── api_utils.rs          # API URL construction
├── date_utils.rs         # Date/time formatting
├── list_utils.rs         # List operations, number formatting
├── clipboard.rs          # Browser clipboard operations
├── export.rs             # Data export utilities
├── icons.rs              # Icon helpers
├── table_utils.rs        # Table operations
├── components/           # Reusable UI components
├── data/                 # Data layer utilities
├── excel_importer/       # Excel import functionality
└── ... (other modules)
```

**What Works:**

- Clear separation by responsibility
- Logical naming makes functions findable
- Each module focused on specific domain

**Improvement Needed:**

- Better documentation of what's available
- Examples in each module's doc comments
- Index file listing all utilities

### 3. Public Functions Should Be Discoverable

**Problem:**
Functions were public but not discoverable:

```rust
// In shared/api_utils.rs - public but unknown
pub fn api_base() -> String { ... }
pub fn api_url(path: &str) -> String { ... }
```

**Solution Pattern:**

1. Add comprehensive module-level documentation
2. Include usage examples in doc comments
3. Consider creating a "shared utilities catalog"

**Example of Good Documentation:**

````rust
//! API utilities for frontend-backend communication
//!
//! Provides helper functions for constructing API URLs and making requests.
//!
//! # Examples
//!
//! ```rust
//! use crate::shared::api_utils::{api_base, api_url};
//!
//! let base = api_base();  // "http://localhost:3000"
//! let url = api_url("/api/users/123");  // full URL
//! ```

/// Get the base URL for API requests
///
/// Constructs the API base URL from the current window location,
/// using port 3000 for the backend server.
pub fn api_base() -> String { ... }
````

### 4. Variants Are Better Than Flags

**Discovery:**
Different date format implementations needed different behavior:

- ISO 8601 with "T" separator: `2025-01-13T10:30:00Z`
- Space-separated format: `2025-01-13 10:30:00`

**Wrong Approach:**

```rust
pub fn format_datetime(datetime_str: &str, use_space: bool) -> String {
    // Branches based on flag
}
```

**Better Approach:**

```rust
pub fn format_datetime(datetime_str: &str) -> String {
    // Handles ISO 8601 format with 'T'
}

pub fn format_datetime_space(datetime_str: &str) -> String {
    // Handles space-separated format
}
```

**Benefit:**

- Self-documenting function names
- No runtime branches
- Easier to test each variant independently
- Clear which variant to use

### 5. Test Coverage is Essential

**Best Practice from Session:**
When enhancing shared utilities, immediately add tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_datetime() {
        assert_eq!(
            format_datetime("2024-03-15T14:02:26.123Z"),
            "15.03.2024 14:02:26"
        );
    }

    #[test]
    fn test_format_datetime_with_z() {
        // Test edge cases
        assert_eq!(
            format_datetime("2025-12-23T16:30:15Z"),
            "23.12.2025 16:30:15"
        );
    }

    #[test]
    fn test_format_datetime_space() {
        assert_eq!(
            format_datetime_space("2025-10-11 00:00:00"),
            "11.10.2025 00:00"
        );
    }
}
```

**Why This Matters:**

- Prevents regressions when enhancing functions
- Documents expected behavior
- Catches edge cases early
- Gives confidence during refactoring

### 6. Incremental Refactoring Reduces Risk

**Strategy That Worked:**

- Group related files (by domain prefix)
- Refactor 5-7 files at a time
- Check compilation after each group
- Proceed to next group only after success

**Example Grouping:**

```
Group 1: a001_connection_1c (2 files)  ✓ Compile
Group 2: a002_organization (2 files)   ✓ Compile
Group 3: a003_counterparty (2 files)   ✓ Compile
Group 4: a004_nomenclature (4 files)   ✓ Compile
...
```

**Why This Works:**

- Errors are contained to small groups
- Easy to identify which change caused issue
- Progress is visible and measurable
- Can pause/resume work easily

### 7. Import Patterns Should Be Consistent

**Good Pattern:**

```rust
// At top of file with other imports
use crate::shared::api_utils::api_base;
use crate::shared::date_utils::format_datetime;
use crate::shared::list_utils::format_number;
```

**Anti-Pattern:**

```rust
// Don't re-implement locally
fn api_base() -> String { ... }

// Don't use qualified paths everywhere
let url = crate::shared::api_utils::api_base();
```

**Style Guide:**

1. Import shared utilities at top of file
2. Use unqualified function calls
3. Group imports by source (shared, domain, external)

## Actionable Recommendations

### Immediate Actions

1. **Create Shared Utilities Index**

   - Document all available shared functions
   - Include usage examples
   - Link from main README

2. **Update `.cursorrules`**

   - Add guideline: "Check `shared` before implementing utilities"
   - Include examples of proper import patterns
   - Reference common utilities

3. **Add Linter Rule** (if possible)
   - Detect duplicate function implementations
   - Suggest shared alternatives

### Long-term Improvements

1. **Code Review Checklist Item**

   - "Does this utility function belong in shared?"
   - "Is there already a shared version?"

2. **Shared Module Documentation**

   - Create markdown file documenting structure
   - Maintain catalog of utilities
   - Include decision criteria for what goes in shared

3. **Periodic Audits**
   - Monthly check for duplicate utilities
   - Refactor as needed
   - Update documentation

## Success Metrics

From this session:

- **24 duplicate functions** consolidated
- **~350 lines** of duplicate code removed
- **Single source of truth** established
- **Zero compilation errors** after refactoring

## Related Documents

- [[RB-code-duplication-detection-v1]]
- [[2025-01-13-session-debrief-code-duplication-refactoring]]
- [[KI-strreplace-fuzzy-matching-2025-01-13]]

## Application to Future Work

**When creating new utilities:**

1. Ask: "Will this be reused?"
2. If yes → put in `shared` immediately
3. Make it public and well-documented
4. Add tests before using

**When finding duplicates:**

1. Follow [[RB-code-duplication-detection-v1]]
2. Group files for systematic refactoring
3. Check compilation incrementally
4. Update documentation

**When onboarding new developers:**

- Share this lesson
- Tour the shared module structure
- Emphasize checking shared first
