//! Brief view of the marketplace navigator: dense grid of icon+label links.

use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::navigator::marketplace::data::{BLOCKS, COLUMNS};
use crate::navigator::marketplace::{link_matches, link_visible};
use crate::navigator::shared::types::{MarketplaceColumn, NavBlock, NavLink};
use crate::shared::components::popover::{hide_nav_tooltip, show_nav_tooltip};
use crate::shared::icons::icon;
use crate::system::auth::context::AuthState;
use leptos::prelude::*;

#[component]
pub fn MarketplaceBrief(
    search: ReadSignal<String>,
    ctx: AppGlobalContext,
    auth_state: ReadSignal<AuthState>,
) -> impl IntoView {
    view! {
        <ColumnHeads />
        <div class="navigator-mp__grid">
            {move || {
                let needle = search.get();
                let visible_blocks: Vec<&'static NavBlock> = BLOCKS
                    .iter()
                    .filter(|block| {
                        block.links.iter().any(|link| {
                            link_visible(auth_state, link) && link_matches(link, &needle)
                        })
                    })
                    .collect();

                if visible_blocks.is_empty() {
                    return view! {
                        <div class="navigator__empty">
                            "Ничего не найдено по запросу «" {needle.clone()} "»"
                        </div>
                    }.into_any();
                }

                visible_blocks
                    .into_iter()
                    .map(|block| {
                        let needle = needle.clone();
                        view! {
                            <BriefBlockRow
                                block=block
                                needle=needle
                                ctx=ctx
                                auth_state=auth_state
                            />
                        }
                    })
                    .collect_view()
                    .into_any()
            }}
        </div>
    }
}

#[component]
fn ColumnHeads() -> impl IntoView {
    view! {
        <div class="navigator-mp__columns-head">
            {COLUMNS
                .iter()
                .map(|col| {
                    let logo: String = col.logo_svg.to_string();
                    view! {
                        <div
                            class="navigator-mp__column-head"
                            style=format!("--mp-color: {};", col.brand_color)
                        >
                            <div
                                class="navigator-mp__logo"
                                data-mp=col.mp_key
                                inner_html=logo
                            ></div>
                            <div class="navigator-mp__column-label">{col.label}</div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn BriefBlockRow(
    block: &'static NavBlock,
    needle: String,
    ctx: AppGlobalContext,
    auth_state: ReadSignal<AuthState>,
) -> impl IntoView {
    let block_id = block.id;
    view! {
        <div
            class="navigator-mp__block"
            data-block=block_id
        >
            <div class="navigator-mp__block-title form__label">
                //<span class="navigator-mp__block-title-icon">{icon(block.icon)}</span>
                <span>{block.label}</span>
            </div>
            {COLUMNS
                .iter()
                .map(|col| {
                    let needle_for_col = needle.clone();
                    view! {
                        <BriefCell
                            block=block
                            column=col
                            needle=needle_for_col
                            ctx=ctx
                            auth_state=auth_state
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn BriefCell(
    block: &'static NavBlock,
    column: &'static MarketplaceColumn,
    needle: String,
    ctx: AppGlobalContext,
    auth_state: ReadSignal<AuthState>,
) -> impl IntoView {
    let links: Vec<&'static NavLink> = block
        .links
        .iter()
        .filter(|link| link.marketplaces.includes(column.kind))
        .filter(|link| link_visible(auth_state, link))
        .filter(|link| link_matches(link, &needle))
        .collect();

    let is_empty = links.is_empty();
    let brand = column.brand_color;

    view! {
        <div
            class="navigator-mp__cell"
            class:navigator-mp__cell--empty=move || is_empty
            style=format!("--mp-color: {};", brand)
        >
            {links
                .into_iter()
                .map(|link| {
                    view! {
                        <BriefLink link=link ctx=ctx />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn BriefLink(link: &'static NavLink, ctx: AppGlobalContext) -> impl IntoView {
    let tab_key = link.tab_key;
    let annotation = link.annotation;
    let on_click = move |_| {
        let label = resolve_label(link);
        ctx.open_tab(tab_key, &label);
    };
    view! {
        <a
            class="navigator-mp__link navigator-mp__link--brief"
            href="#"
            on:click=move |ev| {
                ev.prevent_default();
                on_click(());
            }
            on:mouseenter=move |ev| {
                show_nav_tooltip(annotation, ev.client_x(), ev.client_y());
            }
            on:mouseleave=|_| {
                hide_nav_tooltip();
            }
        >
            <span class="navigator-mp__link-icon">{icon(link.icon)}</span>
            <span class="navigator-mp__link-label">{link.label}</span>
        </a>
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
