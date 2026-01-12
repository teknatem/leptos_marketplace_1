---
type: lesson-learned
date: 2025-01-12
tags: [refactoring, mvvm, architecture]
---

# Lesson: MVVM Refactoring Steps for Detail Forms

## Context

Refactoring monolithic detail form (1200+ lines) into modular MVVM structure.

## Key Learnings

### 1. File Structure Order

Create files in this order to minimize compilation errors:

1. `model.rs` - DTOs and API functions (no dependencies)
2. `view_model.rs` - imports from model
3. `tabs/*.rs` - imports from view_model
4. `page.rs` - imports tabs and view_model
5. `mod.rs` - just exports

### 2. ViewModel Signal Patterns

For read-only forms:

- `RwSignal<Option<MainDto>>` for main data
- Lazy loading signals: `*_loaded: RwSignal<bool>`, `*_loading: RwSignal<bool>`
- Derived signals for computed values

For editable forms:

- Individual `RwSignal<String>` for each field
- Validation in ViewModel methods
- `to_dto()` method for saving

### 3. Tab Lazy Loading

```rust
Effect::new({
    let vm = vm.clone();
    move || {
        match vm.active_tab.get() {
            "tab_name" if !vm.tab_data_loaded.get() => vm.load_tab_data(),
            _ => {}
        }
    }
});
```

### 4. Design After Structure

First get the MVVM structure compiling, then fix the design. Mixing both causes more iteration.

## Anti-Patterns Avoided

- ❌ Inline API calls in Effect
- ❌ `signal()` old style (use `RwSignal::new()`)
- ❌ Moving closures in reactive context without Callback
- ❌ Mixing Card layouts with form\_\_group inconsistently
