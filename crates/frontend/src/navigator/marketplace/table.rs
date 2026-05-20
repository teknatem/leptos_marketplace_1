//! Table view of the marketplace navigator.
//!
//! Flat sortable list: one row per NavLink. Columns: #, Раздел, Название,
//! Тип, Описание, Маркетплейс. All sortable.

use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::navigator::marketplace::data::BLOCKS;
use crate::navigator::marketplace::link_matches;
use crate::navigator::shared::types::{EntityType, LinkScope, MarketplaceKind, NavBlock, NavLink};
use crate::shared::icons::icon;
use crate::system::auth::context::{has_read_access, AuthState};
use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SortField {
    Default,
    Block,
    Name,
    Type,
    Marketplace,
}

#[component]
pub fn MarketplaceTable(
    search: ReadSignal<String>,
    ctx: AppGlobalContext,
    auth_state: ReadSignal<AuthState>,
) -> impl IntoView {
    let sort_field = RwSignal::new(SortField::Default);
    let sort_asc = RwSignal::new(true);

    let toggle_sort = move |field: SortField| {
        if sort_field.get() == field {
            sort_asc.update(|a| *a = !*a);
        } else {
            sort_field.set(field);
            sort_asc.set(true);
        }
    };

    view! {
        <div class="navigator-mp__table-wrapper">
            <table class="navigator-mp__table">
                <colgroup>
                    <col class="col-num" />
                    <col class="col-name" />
                    <col class="col-block" />
                    <col class="col-type" />
                    <col class="col-desc" />
                    <col class="col-mp-single" />
                </colgroup>
                <thead>
                    <tr>
                        <th class="navigator-mp__td--num">"#"</th>
                        <SortTh label="Название"     field=SortField::Name        sort_field=sort_field sort_asc=sort_asc on_click=toggle_sort />
                        <SortTh label="Раздел"       field=SortField::Block       sort_field=sort_field sort_asc=sort_asc on_click=toggle_sort />
                        <SortTh label="Тип"          field=SortField::Type        sort_field=sort_field sort_asc=sort_asc on_click=toggle_sort />
                        <th>"Описание"</th>
                        <SortTh label="Маркетплейс"  field=SortField::Marketplace sort_field=sort_field sort_asc=sort_asc on_click=toggle_sort />
                    </tr>
                </thead>
                <tbody>
                    {move || render_rows(
                        &search.get(),
                        sort_field.get(),
                        sort_asc.get(),
                        auth_state,
                        ctx,
                    )}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn SortTh(
    label: &'static str,
    field: SortField,
    sort_field: RwSignal<SortField>,
    sort_asc: RwSignal<bool>,
    on_click: impl Fn(SortField) + 'static + Copy,
) -> impl IntoView {
    view! {
        <th
            class="navigator-mp__th--sortable"
            on:click=move |_| on_click(field)
        >
            <span class="navigator-mp__th-inner">
                {label}
                <span
                    class="navigator-mp__sort-indicator"
                    class:navigator-mp__sort-indicator--active=move || sort_field.get() == field
                >
                    {move || if sort_field.get() != field { "↕" }
                             else if sort_asc.get() { "▲" }
                             else { "▼" }}
                </span>
            </span>
        </th>
    }
}

fn mp_label(scope: &LinkScope) -> &'static str {
    match scope {
        LinkScope::All => "Все МП",
        LinkScope::Only(list) => match list {
            l if l.contains(&MarketplaceKind::Wildberries) && l.len() == 1 => "Wildberries",
            l if l.contains(&MarketplaceKind::Ozon) && l.len() == 1 => "Ozon",
            l if l.contains(&MarketplaceKind::YandexMarket) && l.len() == 1 => "Яндекс Маркет",
            _ => "Несколько",
        },
    }
}

fn collect_rows(
    needle: &str,
    auth_state: ReadSignal<AuthState>,
) -> Vec<(&'static NavBlock, &'static NavLink)> {
    BLOCKS
        .iter()
        .flat_map(|block| {
            block
                .links
                .iter()
                .filter(|link| link_matches(link, needle))
                .filter(|link| match link.scope_id {
                    None => true,
                    Some(scope) => has_read_access(auth_state, scope),
                })
                .map(move |link| (block, link))
        })
        .collect()
}

fn render_rows(
    needle: &str,
    sort_field: SortField,
    sort_asc: bool,
    auth_state: ReadSignal<AuthState>,
    ctx: AppGlobalContext,
) -> AnyView {
    let mut rows = collect_rows(needle, auth_state);

    match sort_field {
        SortField::Default => {}
        SortField::Block => rows.sort_by(|(a, _), (b, _)| {
            let c = a.label.to_lowercase().cmp(&b.label.to_lowercase());
            if sort_asc {
                c
            } else {
                c.reverse()
            }
        }),
        SortField::Name => rows.sort_by(|(_, a), (_, b)| {
            let c = a.label.to_lowercase().cmp(&b.label.to_lowercase());
            if sort_asc {
                c
            } else {
                c.reverse()
            }
        }),
        SortField::Type => rows.sort_by(|(_, a), (_, b)| {
            let c = a.entity_type.label().cmp(b.entity_type.label());
            if sort_asc {
                c
            } else {
                c.reverse()
            }
        }),
        SortField::Marketplace => rows.sort_by(|(_, a), (_, b)| {
            let c = mp_label(&a.marketplaces).cmp(mp_label(&b.marketplaces));
            if sort_asc {
                c
            } else {
                c.reverse()
            }
        }),
    }

    if rows.is_empty() {
        return view! {
            <tr>
                <td colspan="6">
                    <div class="navigator__empty">
                        "Ничего не найдено по запросу «" {needle.to_string()} "»"
                    </div>
                </td>
            </tr>
        }
        .into_any();
    }

    rows.into_iter()
        .enumerate()
        .map(|(n, (block, link))| {
            view! { <FlatRow num=n+1 block=block link=link ctx=ctx auth_state=auth_state /> }
        })
        .collect_view()
        .into_any()
}

#[component]
fn FlatRow(
    num: usize,
    block: &'static NavBlock,
    link: &'static NavLink,
    ctx: AppGlobalContext,
    auth_state: ReadSignal<AuthState>,
) -> impl IntoView {
    let _ = auth_state;
    let tab_key = link.tab_key;
    let open = move |_: web_sys::MouseEvent| {
        let label = resolve_label(link);
        ctx.open_tab(tab_key, &label);
    };

    let type_label = link.entity_type.label();
    let type_badge = entity_badge_class(link.entity_type);
    let mp_text = mp_label(&link.marketplaces);

    view! {
        <tr class="navigator-mp__row">
            <td class="navigator-mp__td--num">{num}</td>
            <td class="navigator-mp__td--name">
                <a href="#" on:click=move |ev| { ev.prevent_default(); open(ev.clone()); }>
                    {icon(link.icon)}
                    {link.label}
                </a>
            </td>
            <td>
                <span class="navigator-mp__cell-label">{block.label}</span>
            </td>
            <td>
                <span class=type_badge>{type_label}</span>
            </td>
            <td class="navigator-mp__td--desc">{link.annotation}</td>
            <td class="navigator-mp__td--mp-text">{mp_text}</td>
        </tr>
    }
}

fn entity_badge_class(entity_type: EntityType) -> &'static str {
    match entity_type {
        EntityType::Aggregate => "badge badge--primary",
        EntityType::Projection => "badge badge--success",
        EntityType::UseCase => "badge badge--warning",
    }
}

fn resolve_label(link: &'static NavLink) -> String {
    let from_registry = tab_label_for_key(link.tab_key);
    if from_registry.is_empty() {
        link.label.to_string()
    } else {
        from_registry.to_string()
    }
}
