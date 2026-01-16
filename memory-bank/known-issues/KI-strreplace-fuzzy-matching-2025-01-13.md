---
title: Known Issue - StrReplace Fuzzy Matching Failures
discovered: 2025-01-13
severity: medium
status: active
tags: [known-issue, tooling, string-replacement]
---

# Known Issue: StrReplace Fuzzy Matching Failures

## Problem Description

The `StrReplace` tool sometimes fails to match strings due to whitespace or formatting differences, even when the content appears identical. This particularly affects removal of multi-line functions during refactoring.

## Symptoms

1. Replacement operation returns error: "The string to replace was not found in the file"
2. Tool suggests "Found a possible fuzzy match"
3. Function visually appears to match but replacement fails
4. Multiple retry attempts with same text continue to fail

## Root Cause

The tool requires **exact character-by-character matching** including:

- Whitespace (spaces vs tabs)
- Line endings
- Indentation levels
- Comments or blank lines within the matched section

## Detection

When you see error message:

```
Error: The string to replace was not found in the file (even after relaxing whitespace).
Found a possible fuzzy match, did you mean: ...
```

This indicates the match is close but not exact.

## Solution Strategy

### Method 1: Read Exact Context (Preferred)

1. Read the file with sufficient context around the target:

   ```
   Read file at lines N-10 to N+20
   ```

2. Copy the **exact text** including all whitespace:

   - Use the actual output from the Read tool
   - Include 3-5 lines before and after the change point
   - Preserve all indentation exactly

3. Match the entire block including context

### Method 2: Increase Context Window

If first attempt fails:

- Read more lines before and after
- Include unique identifying lines (function names, struct definitions)
- Ensure the matched section is truly unique in the file

### Method 3: Target Smaller Sections

Instead of replacing entire function:

1. Replace just the function signature line
2. Then replace the function body
3. Avoid matching across large line ranges

## Example from Session

**Failed Attempt:**

```rust
// Tried to match:
use contracts::domain::a002_organization::aggregate::{Organization, OrganizationDto};

fn api_base() -> String {
    // ... function body ...
}

pub async fn fetch_by_id(id: String) -> Result<Organization, String> {
```

**Reason for Failure:**
The actual file had additional imports or different formatting that wasn't visible in the initial read.

**Successful Approach:**

```rust
// Read exact text from file output:
use contracts::domain::a002_organization::aggregate::{Organization, OrganizationDto};

fn api_base() -> String {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return String::new(),
    };
    let location = window.location();
    let protocol = location.protocol().unwrap_or_else(|_| "http:".to_string());
    let hostname = location
        .hostname()
        .unwrap_or_else(|_| "127.0.0.1".to_string());
    format!("{}//{}:3000", protocol, hostname)
}

pub async fn fetch_by_id(id: String) -> Result<Organization, String> {
```

Included complete function body and surrounding context.

## Prevention

### Best Practices

1. **Always read before replacing**

   - Use Read tool to get exact text
   - Don't assume formatting

2. **Include sufficient context**

   - Minimum 3-5 lines before and after
   - Include unique identifiers (struct names, other function names)

3. **Verify uniqueness**

   - Ensure matched text appears only once in file
   - Use grep to check: `rg "pattern" filename --count`

4. **Start small, expand if needed**

   - Begin with minimal context
   - Add more context if match fails
   - Document the working pattern for similar cases

5. **Test incrementally**
   - Replace one function at a time
   - Verify compilation after each replacement
   - Don't batch too many operations

## Workaround Pattern

```
Step 1: Read target section
  Read(path, offset=N, limit=20)

Step 2: Copy exact output including line numbers prefix

Step 3: Remove line number prefix for matching

Step 4: Include context that makes match unique
  - Previous function end
  - Next function start
  - Unique comments or struct definitions

Step 5: Perform replacement
  StrReplace(path, old_string=<exact_text>, new_string=<new_text>)

Step 6: If fails, read more context and retry
```

## Impact

- **Frequency**: Occurs in ~10-15% of refactoring operations
- **Impact**: Delays refactoring work, requires manual retries
- **Severity**: Medium - annoying but has reliable workaround

## Related Issues

- Similar issue affects any multi-line text replacement
- Particularly common when:
  - Removing complete functions
  - Replacing large code blocks
  - Working with files that have mixed indentation

## Related Documents

- [[RB-code-duplication-detection-v1]] - Shows workaround in practice
- [[2025-01-13-session-debrief-code-duplication-refactoring]] - Examples from actual session
