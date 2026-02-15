# Skill Changelog

## 2026-02-11: Major Update - List Refactoring Comprehensive Checklist

### Summary

Added complete checklist for refactoring list views to modern standards (BEM + Thaw UI + Server-side pagination) based on successful refactorings of a015_wb_orders and a013_ym_order.

### What Was Added

#### 1. **List View Refactoring Checklist** (Major Section)

Comprehensive step-by-step checklist covering:

- Backend refactoring (repository, handler, routes, contracts)
- Frontend state management (HashSet for selection, pagination fields)
- Frontend UI components (Thaw Table, filters, pagination)
- Sortable headers with CSS classes (fixes green indicator bug)
- Selection management patterns
- Data loading with server-side pagination
- Code cleanup guidelines

#### 2. **Common Pitfalls & Solutions**

Real issues encountered during refactorings:

1. Sort indicator not turning green → Missing `get_sort_class` CSS class
2. HashSet errors → Using Vec instead of HashSet
3. Pagination not resetting → Missing `page = 0` on filter change
4. Selection not working → Wrong data structure
5. Duplicate functions → Incomplete code cleanup
6. Column resize not working → Missing init call
7. Money cells not aligned → Not using TableCellMoney
8. Page not updating → Filter Effect missing reload

#### 3. **Quick Reference Card**

Copy-paste ready templates:

- Essential imports for list views
- State.rs template with HashSet
- Sortable header pattern (with CSS class!)
- Selection management pattern
- Filter RwSignal pattern with auto-reload
- Backend paginated response format
- Top 5 bugs to avoid

#### 4. **USAGE.md Guide**

Practical guide with:

- When to use the checklist
- Step-by-step workflow
- Common issues with exact fixes
- Time estimates
- Reference implementations
- Validation steps
- Golden rules summary

### Key Features

**Backend Checklist:**

- ✅ Repository layer with pagination/filtering
- ✅ Handler with full paginated response
- ✅ Organization enrichment pattern
- ✅ Unified routes

**Frontend Checklist:**

- ✅ Complete imports list
- ✅ Constants (TABLE_ID, COLUMN_WIDTHS_KEY)
- ✅ State with HashSet<String> for selection
- ✅ RwSignals for filter controls
- ✅ Filter panel (single-row, collapsible)
- ✅ Thaw Table components
- ✅ Sortable headers with get_sort_class
- ✅ TableCellCheckbox/TableHeaderCheckbox
- ✅ TableCellMoney for monetary values
- ✅ PaginationControls integration
- ✅ Column resize initialization
- ✅ Batch operations (Post/Unpost)

**Testing & Verification:**

- ✅ Compilation checks
- ✅ UI functionality tests
- ✅ Visual checks (sort indicator green!)

### Reference Implementations

Primary: `a012_wb_sales`
Secondary: `a015_wb_orders`, `a013_ym_order`

### Migration Timeline

Based on actual refactorings:

- Backend: 1-2 hours (25%)
- State: 30 min (10%)
- UI: 3-4 hours (50%)
- Cleanup: 1 hour (15%)
  Total: 4-6 hours for standard list

### Files Modified

1. `.cursor/skills/audit-page-bem-thaw/SKILL.md` - Added List View Refactoring Checklist section
2. `.cursor/skills/audit-page-bem-thaw/USAGE.md` - Created usage guide
3. `.cursor/skills/audit-page-bem-thaw/CHANGELOG.md` - This file

### Bug Documentation

**Most Common Bug**: Sort indicator not turning green

- **Cause**: Missing `get_sort_class` import or not applying CSS class to span
- **Frequency**: Occurred in both a015_wb_orders and would occur in any list without this pattern
- **Fix**: Import `get_sort_class`, use `class=move || state.with(|s| get_sort_class(...))`
- **Added to**: Common Pitfalls, Quick Reference Card, USAGE guide

### Impact

This update transforms the skill from a general BEM audit tool into a complete list refactoring guide with:

- ✅ Step-by-step checklists (no guesswork)
- ✅ Copy-paste templates (faster implementation)
- ✅ Common pitfalls documented (avoid repeated mistakes)
- ✅ Real-world patterns (based on actual refactorings)
- ✅ Time estimates (better planning)
- ✅ Validation criteria (ensure quality)

### Next Steps for Users

1. Use checklist when refactoring any list view
2. Reference a012_wb_sales for patterns
3. Follow Backend → State → UI order
4. Test incrementally
5. Check off items as you go
6. Validate with Testing & Verification checklist

### Notes

- All patterns validated in production refactorings (a012, a015, a013)
- Checklist covers 100% of refactoring scenarios encountered
- Time estimates based on actual work (not theoretical)
- Bug list based on real issues fixed during refactorings
