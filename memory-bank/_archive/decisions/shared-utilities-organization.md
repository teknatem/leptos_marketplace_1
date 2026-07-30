---
title: ADR-0003 - Shared Utilities Organization
date: 2025-01-13
status: accepted
tags: [adr, architecture, shared-utilities]
---

# ADR-0003: Shared Utilities Organization

## Status

**ACCEPTED** - 2025-01-13

## Context

During code duplication refactoring, we discovered:

- 17 duplicate `api_base()` implementations
- 6 duplicate `format_datetime()` implementations
- 1 duplicate `format_number_with_separator()` implementation

The `shared` module already existed but was underutilized. Only 1 out of 18 modules correctly imported utilities from shared.

## Decision

We establish `crates/frontend/src/shared/` as the **single source of truth** for all reusable utility functions in the frontend.

### Organizational Principles

1. **All reusable utilities go in shared module**

   - API utilities → `shared/api_utils.rs`
   - Date/time utilities → `shared/date_utils.rs`
   - Number formatting → `shared/list_utils.rs`
   - Browser operations → `shared/clipboard.rs`, `shared/export.rs`
   - UI helpers → `shared/components/`

2. **Functions must be public and documented**

   ````rust
   /// Clear description of what function does
   ///
   /// # Arguments
   /// * `param` - Description
   ///
   /// # Returns
   /// Description of return value
   ///
   /// # Example
   /// ```rust
   /// let result = utility_function(input);
   /// ```
   pub fn utility_function(param: Type) -> Return { ... }
   ````

3. **Create variants instead of flags**

   - Good: `format_datetime()` and `format_datetime_space()`
   - Bad: `format_datetime(str, use_space: bool)`

4. **Test coverage required**

   - Every public function needs tests
   - Cover edge cases and error conditions
   - Document behavior through tests

5. **No local reimplementations**
   - If utility exists in shared, use it
   - If you need to implement a utility, add it to shared first
   - Exception: Domain-specific logic that truly belongs in domain module

## Alternatives Considered

### Alternative 1: Allow Duplicate Implementations

**Pros:**

- Each module is self-contained
- No coupling to shared module
- Can optimize for specific use case

**Cons:**

- Maintenance nightmare (need to update in multiple places)
- Inconsistent behavior across modules
- More code to maintain and test
- Harder to enforce standards

**Decision:** Rejected - maintenance cost too high

### Alternative 2: Create Utility Crate

**Pros:**

- Clear separation of utilities
- Could be shared across multiple projects
- Forced public API design

**Cons:**

- Overhead of managing separate crate
- Slower development (need to publish/update crate)
- Overkill for single-project utilities

**Decision:** Rejected - premature abstraction for our needs

### Alternative 3: Domain-Specific Utility Modules

**Pros:**

- Utilities grouped with related domain logic
- Clearer context for utility usage

**Cons:**

- Unclear where cross-domain utilities go
- Encourages duplication across domains
- Harder to discover available utilities

**Decision:** Rejected - leads to same duplication problem

## Consequences

### Positive

1. **Single Source of Truth**

   - Changes in one place affect all consumers
   - Consistent behavior across application
   - Easier to maintain and update

2. **Reduced Code Duplication**

   - Removed ~350 lines of duplicate code
   - Future utilities start in shared
   - Less code to test and maintain

3. **Improved Discoverability**

   - Clear location for utilities
   - Better documentation
   - New developers know where to look

4. **Better Testing**
   - Test once, benefit everywhere
   - More comprehensive test coverage
   - Easier to add edge case tests

### Negative

1. **Increased Coupling**

   - All modules depend on shared
   - Breaking changes in shared affect many files
   - Mitigation: Careful API design, deprecation process

2. **Potential Performance Impact**

   - Generic implementation may not be optimal for all cases
   - Mitigation: Profile and optimize if needed, allow overrides for critical paths

3. **More Upfront Design**
   - Need to think about API before implementing
   - Takes slightly longer initially
   - Mitigation: Start simple, refine as needed

## Implementation

### Phase 1: Consolidation (Completed)

- [x] Enhanced `shared/api_utils.rs` with `api_base()`
- [x] Enhanced `shared/date_utils.rs` with `format_datetime()` variants
- [x] Used `shared/list_utils.rs` for number formatting
- [x] Removed 24 duplicate implementations
- [x] Added comprehensive tests

### Phase 2: Documentation (In Progress)

- [x] Created runbook for detecting duplicates
- [ ] Create shared utilities catalog
- [ ] Update project README with shared module overview
- [ ] Add examples to `.cursorrules`

### Phase 3: Enforcement (Planned)

- [ ] Add code review checklist item
- [ ] Consider linting rule for duplicates
- [ ] Periodic audits (monthly)
- [ ] Developer onboarding includes shared module tour

## Monitoring

We will track:

- Number of new utilities added to shared
- Instances of duplicate implementations (should trend to zero)
- Developer feedback on shared utilities
- Performance metrics of shared implementations

## Related Documents

- [[RB-code-duplication-detection-v1]] - Runbook for implementation
- [[LL-shared-module-organization-2025-01-13]] - Lessons learned
- [[2025-01-13-session-debrief-code-duplication-refactoring]] - Session that led to this decision

## Review Schedule

This decision will be reviewed in 3 months (April 2025) to assess:

- Effectiveness of shared module organization
- Developer satisfaction
- Any emerging patterns that suggest refinement
