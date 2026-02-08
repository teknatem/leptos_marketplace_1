# Frontend Page Templates

_Last updated: 2026-01-30_

Ready-to-use templates for creating new frontend pages that follow BEM + Thaw UI standards.

## Template 1: Simple List View

**Use for:** Simple CRUD lists without filters or complex interactions

**Reference implementation:** `a002_organization/ui/list/mod.rs`

```rust
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use crate::shared::modal_stack::ModalStackService;
use contracts::domain::aXXX_entity::aggregate::Entity;
use leptos::prelude::*;
use std::collections::HashSet;

// Import details component
use super::details::EntityDetails;

#[component]
pub fn EntityList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let modal_stack = use_context::<ModalStackService>().expect("ModalStackService not found");
    
    let (items, set_items) = signal::<Vec<Entity>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);
    let (selected, set_selected) = signal::<HashSet<String>>(HashSet::new());
    let (editing_id, set_editing_id) = signal::<Option<String>>(None);
    let show_modal = RwSignal::new(false);

    // Fetch data from API
    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match gloo_net::http::Request::get("http://localhost:3000/api/aXXX/entity")
                .send()
                .await
            {
                Ok(response) => {
                    if response.ok() {
                        match response.json::<Vec<Entity>>().await {
                            Ok(data) => {
                                set_items.set(data);
                                set_error.set(None);
                            }
                            Err(e) => set_error.set(Some(format!("Parse error: {}", e))),
                        }
                    } else {
                        set_error.set(Some(format!("HTTP {}", response.status())));
                    }
                }
                Err(e) => set_error.set(Some(format!("Network error: {}", e))),
            }
        });
    };

    // Create new entity
    let handle_create_new = move || {
        set_editing_id.set(None);
        show_modal.set(true);
    };

    // Edit existing entity
    let handle_edit = move |id: String| {
        set_editing_id.set(Some(id));
        show_modal.set(true);
    };

    // Delete selected entities
    let delete_selected = move || {
        let selected_ids = selected.get();
        if selected_ids.is_empty() {
            return;
        }
        
        // TODO: Show confirmation dialog
        // TODO: Call delete API
        // TODO: Refresh list
    };

    // Initial fetch
    fetch();

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Сущности"}</h1>
                </div>
                <div class="page__header-right">
                    <button class="button button--primary" on:click=move |_| handle_create_new()>
                        {icon("plus")}
                        {"Новая сущность"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                    <button 
                        class="button button--secondary" 
                        on:click=move |_| delete_selected() 
                        disabled={move || selected.get().is_empty()}
                    >
                        {icon("delete")}
                        {move || format!("Удалить ({})", selected.get().len())}
                    </button>
                </div>
            </div>

            {move || error.get().map(|e| view! {
                <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100);">
                    <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                    <span class="warning-box__text" style="color: var(--color-error);">{e}</span>
                </div>
            })}

            <div class="table">
                <table class="table__data table--striped">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell table__header-cell--checkbox">
                                <input
                                    type="checkbox"
                                    class="table__checkbox"
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        let current_items = items.get();
                                        if checked {
                                            set_selected.update(|s| {
                                                for item in current_items.iter() {
                                                    s.insert(item.id.clone());
                                                }
                                            });
                                        } else {
                                            set_selected.set(HashSet::new());
                                        }
                                    }
                                />
                            </th>
                            <th class="table__header-cell">{"Код"}</th>
                            <th class="table__header-cell">{"Наименование"}</th>
                            <th class="table__header-cell">{"Комментарий"}</th>
                            <th class="table__header-cell">{"Создано"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || items.get().into_iter().map(|row| {
                            let id = row.id.clone();
                            let id_for_click = id.clone();
                            let id_for_checkbox = id.clone();
                            let is_selected = selected.get().contains(&id);

                            view! {
                                <tr 
                                    class="table__row" 
                                    class:table__row--selected=is_selected
                                    on:click=move |_| handle_edit(id_for_click.clone())
                                >
                                    <td class="table__cell table__cell--checkbox">
                                        <input
                                            type="checkbox"
                                            class="table__checkbox"
                                            prop:checked=is_selected
                                            on:change=move |ev| {
                                                ev.stop_propagation();
                                                let checked = event_target_checked(&ev);
                                                set_selected.update(|s| {
                                                    if checked {
                                                        s.insert(id_for_checkbox.clone());
                                                    } else {
                                                        s.remove(&id_for_checkbox);
                                                    }
                                                });
                                            }
                                        />
                                    </td>
                                    <td class="table__cell">{row.code}</td>
                                    <td class="table__cell">{row.description}</td>
                                    <td class="table__cell">{row.comment.unwrap_or("-".to_string())}</td>
                                    <td class="table__cell">{format_date(&row.created_at)}</td>
                                </tr>
                            }
                        }).collect_view()}
                    </tbody>
                </table>
            </div>

            // Modal for create/edit
            <Show when=move || show_modal.get()>
                {move || {
                    let id_val = editing_id.get();
                    modal_stack.push_with_frame(
                        Some("max-width: 600px; width: 95vw;".to_string()),
                        Some("entity-modal".to_string()),
                        move |handle| {
                            let id_signal = Signal::derive({
                                let id_val = id_val.clone();
                                move || id_val.clone()
                            });

                            view! {
                                <EntityDetails
                                    id=id_signal
                                    on_saved=Callback::new({
                                        let handle = handle.clone();
                                        move |_| {
                                            handle.close();
                                            fetch();
                                        }
                                    })
                                    on_cancel=Callback::new({
                                        let handle = handle.clone();
                                        move |_| handle.close()
                                    })
                                />
                            }.into_any()
                        },
                    );

                    show_modal.set(false);
                    set_editing_id.set(None);
                    view! { <></> }
                }}
            </Show>
        </div>
    }
}

fn format_date(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}
```

## Template 2: Complex List with Filters and Thaw Table

**Use for:** Lists with pagination, filters, sorting, and complex interactions

**Reference implementation:** `a012_wb_sales/ui/list/mod.rs`

```rust
pub mod state;

use self::state::create_state;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::components::ui::button::Button as UiButton;
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[component]
pub fn EntityList() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    
    let state = create_state();
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);
    let (is_filter_expanded, set_is_filter_expanded) = signal(false);

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let date_from = state.with(|s| s.date_from.clone());
            let date_to = state.with(|s| s.date_to.clone());
            let page = state.with(|s| s.page);
            let page_size = state.with(|s| s.page_size);

            // Build API URL with parameters
            let url = format!(
                "http://localhost:3000/api/aXXX/entity?from={}&to={}&limit={}&offset={}",
                date_from, date_to, page_size, page * page_size
            );

            // Fetch from API
            // Parse response
            // Update state

            set_loading.set(false);
        });
    };

    load_items();

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Сущности"}</h1>
                    <UiBadge variant="primary".to_string()>
                        {move || state.get().total_count.to_string()}
                    </UiBadge>
                </div>
                <div class="page__header-right">
                    <Space>
                        <UiButton 
                            variant="primary".to_string()
                            on_click=Callback::new(move |_| { /* action */ })
                        >
                            {icon("plus")}
                            "Создать"
                        </UiButton>
                        <UiButton 
                            variant="secondary".to_string()
                            on_click=Callback::new(move |_| load_items())
                        >
                            {icon("refresh")}
                            "Обновить"
                        </UiButton>
                    </Space>
                </div>
            </div>

            <div class="filter-panel">
                <div class="filter-panel-header">
                    <div 
                        class="filter-panel-header__left"
                        on:click=move |_| set_is_filter_expanded.update(|e| *e = !*e)
                    >
                        <button class="filter-panel-toggle">
                            {icon("filter")}
                            "Фильтры"
                            {icon(if is_filter_expanded.get() { "chevron-up" } else { "chevron-down" })}
                        </button>
                    </div>
                    <div class="filter-panel-header__center">
                        // Optional: filter summary
                    </div>
                    <div class="filter-panel-header__right">
                        <button class="filter-panel-clear" on:click=move |_| { /* clear filters */ }>
                            {icon("x")}
                            "Сбросить"
                        </button>
                    </div>
                </div>

                <Show when=move || is_filter_expanded.get()>
                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Size(16.0) wrap=FlexWrap::Wrap>
                            <DateRangePicker 
                                date_from=Signal::derive(move || state.get().date_from.clone())
                                date_to=Signal::derive(move || state.get().date_to.clone())
                                on_date_from_change=move |val| state.update(|s| s.date_from = val)
                                on_date_to_change=move |val| state.update(|s| s.date_to = val)
                            />
                            
                            <div class="filter-group">
                                <Label>{"Фильтр"}</Label>
                                <Select value=Signal::derive(move || state.get().filter.clone())>
                                    <SelectOption value="all">{"Все"}</SelectOption>
                                    <SelectOption value="active">{"Активные"}</SelectOption>
                                </Select>
                            </div>
                        </Flex>
                    </div>
                </Show>
            </div>

            <div class="page-content">
                <Table>
                    <TableHeader>
                        <TableRow>
                            <TableHeaderCell resizable=false class="fixed-checkbox-column">
                                <input
                                    type="checkbox"
                                    style="cursor: pointer;"
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        // Select all logic
                                    }
                                />
                            </TableHeaderCell>
                            <TableHeaderCell 
                                on:click=move |_| { /* sort by code */ }
                                class=get_sort_class("code", state.get().sort_field.clone(), state.get().sort_ascending)
                            >
                                {"Код"}
                                {get_sort_indicator("code", state.get().sort_field.clone(), state.get().sort_ascending)}
                            </TableHeaderCell>
                            <TableHeaderCell 
                                on:click=move |_| { /* sort by description */ }
                                class=get_sort_class("description", state.get().sort_field.clone(), state.get().sort_ascending)
                            >
                                {"Наименование"}
                                {get_sort_indicator("description", state.get().sort_field.clone(), state.get().sort_ascending)}
                            </TableHeaderCell>
                        </TableRow>
                    </TableHeader>
                    <TableBody>
                        {move || state.get().items.into_iter().map(|row| {
                            let id = row.id.clone();
                            let is_selected = state.get().selected_ids.contains(&id);

                            view! {
                                <TableRow>
                                    <TableCell>
                                        <TableCellLayout>
                                            <input
                                                type="checkbox"
                                                prop:checked=is_selected
                                                on:change=move |ev| {
                                                    ev.stop_propagation();
                                                    // Toggle selection
                                                }
                                            />
                                        </TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout>{row.code}</TableCellLayout>
                                    </TableCell>
                                    <TableCell>
                                        <TableCellLayout truncate=true>{row.description}</TableCellLayout>
                                    </TableCell>
                                </TableRow>
                            }
                        }).collect_view()}
                    </TableBody>
                </Table>

                <PaginationControls 
                    current_page=Signal::derive(move || state.get().page)
                    total_pages=Signal::derive(move || state.get().total_pages)
                    on_page_change=Callback::new(move |page: usize| {
                        state.update(|s| s.page = page);
                        load_items();
                    })
                />
            </div>
        </div>
    }
}
```

**Required state.rs:**

```rust
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct PageState {
    pub items: Vec<Entity>,
    pub selected_ids: HashSet<String>,
    pub date_from: String,
    pub date_to: String,
    pub filter: String,
    pub sort_field: String,
    pub sort_ascending: bool,
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

pub fn create_state() -> RwSignal<PageState> {
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month_ago = (chrono::Utc::now() - chrono::Duration::days(30))
        .format("%Y-%m-%d")
        .to_string();

    RwSignal::new(PageState {
        items: Vec::new(),
        selected_ids: HashSet::new(),
        date_from: month_ago,
        date_to: today,
        filter: "all".to_string(),
        sort_field: "code".to_string(),
        sort_ascending: true,
        page: 0,
        page_size: 50,
        total_count: 0,
        total_pages: 0,
    })
}
```

## Template 3: Tree View

**Use for:** Hierarchical data (counterparties, nomenclature)

**Reference implementation:** `a003_counterparty/ui/tree/widget.rs`

```rust
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct TreeNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub code: String,
    pub description: String,
    pub level: usize,
    pub has_children: bool,
}

#[component]
pub fn EntityTree() -> impl IntoView {
    let ctx = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    
    let (items, set_items) = signal::<Vec<TreeNode>>(Vec::new());
    let (expanded, set_expanded) = signal::<HashSet<String>>(HashSet::new());
    
    let fetch = move || {
        // Fetch tree data
    };

    let toggle_expand = move |id: String| {
        set_expanded.update(|exp| {
            if exp.contains(&id) {
                exp.remove(&id);
            } else {
                exp.insert(id);
            }
        });
    };

    fetch();

    view! {
        <div class="page">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">{"Дерево сущностей"}</h1>
                </div>
                <div class="page__header-right">
                    <button class="button button--primary" on:click=move |_| { /* create */ }>
                        {icon("plus")}
                        {"Добавить"}
                    </button>
                    <button class="button button--secondary" on:click=move |_| fetch()>
                        {icon("refresh")}
                        {"Обновить"}
                    </button>
                </div>
            </div>

            <div class="table">
                <table class="table__data table--tree">
                    <thead class="table__head">
                        <tr>
                            <th class="table__header-cell">{"Код"}</th>
                            <th class="table__header-cell">{"Наименование"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        {move || {
                            let all_items = items.get();
                            let expanded_ids = expanded.get();
                            
                            render_tree_nodes(&all_items, None, &expanded_ids, toggle_expand)
                        }}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn render_tree_nodes(
    items: &[TreeNode],
    parent_id: Option<String>,
    expanded: &HashSet<String>,
    toggle_expand: impl Fn(String) + Clone + 'static,
) -> Vec<impl IntoView> {
    items
        .iter()
        .filter(|item| item.parent_id == parent_id)
        .flat_map(|node| {
            let node_id = node.id.clone();
            let is_expanded = expanded.contains(&node_id);
            let indent = node.level * 20;

            let mut result = vec![view! {
                <tr class="table__row">
                    <td class="table__cell" style:padding-left=format!("{}px", indent)>
                        {node.has_children.then(|| {
                            let id_for_toggle = node_id.clone();
                            view! {
                                <button 
                                    class="tree-toggle"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        toggle_expand(id_for_toggle.clone());
                                    }
                                >
                                    {icon(if is_expanded { "chevron-down" } else { "chevron-right" })}
                                </button>
                            }
                        })}
                        {node.code.clone()}
                    </td>
                    <td class="table__cell">{node.description.clone()}</td>
                </tr>
            }.into_any()];

            if is_expanded && node.has_children {
                let children = render_tree_nodes(items, Some(node_id), expanded, toggle_expand.clone());
                result.extend(children);
            }

            result
        })
        .collect()
}
```

## Template 4: Details/Form View

**Use for:** Create/Edit forms

```rust
use crate::shared::icons::icon;
use leptos::prelude::*;
use thaw::*;

#[component]
pub fn EntityDetails(
    #[prop(into)] id: Signal<Option<String>>,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let (code, set_code) = signal(String::new());
    let (description, set_description) = signal(String::new());
    let (comment, set_comment) = signal(String::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    // Load existing data if editing
    Effect::new(move |_| {
        if let Some(entity_id) = id.get() {
            // Fetch entity data
        }
    });

    let save = move |_| {
        set_loading.set(true);
        set_error.set(None);

        let entity_id = id.get();
        // Save logic (create or update)

        on_saved.call(());
    };

    view! {
        <Card>
            <div class="form">
                <div class="form__header">
                    <h2 class="form__title">
                        {move || if id.get().is_some() { "Редактирование" } else { "Создание" }}
                    </h2>
                </div>

                <div class="form__body">
                    {move || error.get().map(|e| view! {
                        <div class="alert alert--error">
                            {icon("alert-circle")}
                            {e}
                        </div>
                    })}

                    <div class="form__group">
                        <Label>{"Код"}</Label>
                        <Input 
                            value=Signal::derive(move || code.get())
                            on_input=move |val| set_code.set(val)
                            placeholder="Введите код"
                        />
                    </div>

                    <div class="form__group">
                        <Label>{"Наименование"}</Label>
                        <Input 
                            value=Signal::derive(move || description.get())
                            on_input=move |val| set_description.set(val)
                            placeholder="Введите наименование"
                        />
                    </div>

                    <div class="form__group">
                        <Label>{"Комментарий"}</Label>
                        <Textarea 
                            value=Signal::derive(move || comment.get())
                            on_input=move |val| set_comment.set(val)
                            placeholder="Опциональный комментарий"
                            rows=3
                        />
                    </div>
                </div>

                <div class="form__footer">
                    <Space>
                        <Button 
                            appearance=ButtonAppearance::Primary
                            on_click=save
                            loading=Signal::derive(move || loading.get())
                        >
                            {icon("save")}
                            "Сохранить"
                        </Button>
                        <Button 
                            appearance=ButtonAppearance::Secondary
                            on_click=move |_| on_cancel.call(())
                        >
                            {icon("x")}
                            "Отмена"
                        </Button>
                    </Space>
                </div>
            </div>
        </Card>
    }
}
```

## Quick Reference

### Essential Imports

```rust
// Layout
use crate::layout::global_context::AppGlobalContext;

// Icons
use crate::shared::icons::icon;

// Modal
use crate::shared::modal_stack::ModalStackService;

// Thaw UI
use thaw::*;

// List utilities
use crate::shared::list_utils::{get_sort_class, get_sort_indicator, Sortable};

// Custom components
use crate::shared::components::ui::badge::Badge as UiBadge;
use crate::shared::components::ui::button::Button as UiButton;
use crate::shared::components::date_range_picker::DateRangePicker;
use crate::shared::components::pagination_controls::PaginationControls;

// Leptos
use leptos::prelude::*;
use leptos::task::spawn_local;

// Standard library
use std::collections::HashSet;
```

### BEM Class Reference

**Page Structure:**
- `.page` - Root container
- `.page__header` - Sticky header
- `.page__header-left` - Left section
- `.page__header-right` - Right section
- `.page__title` - H1 title
- `.page-content` - Main content area

**Table (Native):**
- `.table` - Container wrapper
- `.table__data` - Table element
- `.table--striped` - Striped modifier
- `.table__head` - Thead
- `.table__header-cell` - Th
- `.table__row` - Tr
- `.table__cell` - Td
- `.table__checkbox` - Checkbox input

**Filter Panel:**
- `.filter-panel` - Container
- `.filter-panel-header` - Header
- `.filter-panel-header__left` - Left section
- `.filter-panel-header__center` - Center section
- `.filter-panel-header__right` - Right section
- `.filter-panel-content` - Content area
- `.filter-panel-toggle` - Toggle button
- `.filter-panel-clear` - Clear button

**Buttons (Native):**
- `.button` - Base button
- `.button--primary` - Primary variant
- `.button--secondary` - Secondary variant
- `.button--ghost` - Ghost variant

## Related Documentation

- `memory-bank/architecture/frontend-page-standards.md` - Detailed standards
- `memory-bank/runbooks/RB-page-refactoring-to-bem-thaw-v1.md` - Refactoring guide
- `.cursor/skills/audit-page-bem-thaw/SKILL.md` - Automated audit tool
