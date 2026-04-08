use crate::general_ledger::api::fetch_general_ledger_turnovers;
use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;
use contracts::general_ledger::GeneralLedgerTurnoverDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn join_or_dash(values: &[String]) -> String {
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

#[component]
pub fn GeneralLedgerTurnoversPage() -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");
    let (items, set_items) = signal(Vec::<GeneralLedgerTurnoverDto>::new());
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let search_query = RwSignal::new(String::new());
    let report_group_filter = RwSignal::new(String::new());
    let only_with_entries = RwSignal::new(false);
    let only_journal_entries = RwSignal::new(false);
    let loaded = RwSignal::new(false);

    let load_items = move || {
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            match fetch_general_ledger_turnovers().await {
                Ok(response) => {
                    set_items.set(response.items);
                    loaded.set(true);
                }
                Err(err) => set_error.set(Some(err)),
            }

            set_loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !loaded.get() {
            load_items();
        }
    });

    let filtered_items = Signal::derive(move || {
        let search = search_query.get().trim().to_lowercase();
        let report_group = report_group_filter.get().trim().to_lowercase();
        let only_with_entries = only_with_entries.get();
        let only_journal_entries = only_journal_entries.get();

        items
            .get()
            .into_iter()
            .filter(|item| {
                if only_with_entries && item.gl_entries_count == 0 {
                    return false;
                }

                if only_journal_entries && !item.generates_journal_entry {
                    return false;
                }

                if !report_group.is_empty()
                    && !item
                        .report_group
                        .as_str()
                        .to_lowercase()
                        .contains(&report_group)
                {
                    return false;
                }

                if search.is_empty() {
                    return true;
                }

                let haystack = format!(
                    "{} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {} {}",
                    item.code,
                    item.name,
                    item.description,
                    item.llm_description,
                    item.formula_hint,
                    item.notes,
                    item.journal_comment,
                    item.debit_account,
                    item.credit_account,
                    item.aliases.join(" "),
                    item.source_examples.join(" "),
                    item.report_group.as_str(),
                    item.scope.as_str(),
                    item.value_kind.as_str(),
                    item.agg_kind.as_str(),
                    item.selection_rule.as_str(),
                    item.sign_policy.as_str(),
                )
                .to_lowercase();

                haystack.contains(&search)
            })
            .collect::<Vec<_>>()
    });

    let filtered_count = Signal::derive(move || filtered_items.get().len());
    let total_count = Signal::derive(move || items.get().len());

    view! {
        <PageFrame
            page_id="general_ledger_turnovers--list"
            category=PAGE_CAT_LIST
            class="page--wide"
        >
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Обороты GL"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {move || filtered_count.get().to_string()}
                    </Badge>
                    <span style="font-size: 12px; opacity: 0.75;">
                        {move || format!("из {}", total_count.get())}
                    </span>
                </div>

                <div class="page__header-right">
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            tabs_store.open_tab("general_ledger", tab_label_for_key("general_ledger"));
                        }
                    >
                        "Журнал GL"
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| {
                            tabs_store.open_tab("general_ledger_report", "Отчёт GL");
                        }
                    >
                        "Отчёт GL"
                    </Button>

                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load_items()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|err| view! {
                    <div class="alert alert--error">{err}</div>
                })}

                <div class="filter-panel">
                    <div class="filter-panel-header">
                        <div class="filter-panel-header__left">
                            <span class="filter-panel__title">"Фильтры"</span>
                        </div>
                    </div>

                    <div class="filter-panel-content">
                        <Flex gap=FlexGap::Small align=FlexAlign::End style="flex-wrap: wrap;">
                            <div style="width: 320px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Поиск"</Label>
                                    <Input value=search_query placeholder="code, name, description, account..." />
                                </Flex>
                            </div>

                            <div style="width: 180px;">
                                <Flex vertical=true gap=FlexGap::Small>
                                    <Label>"Report group"</Label>
                                    <Input value=report_group_filter placeholder="revenue, advertising..." />
                                </Flex>
                            </div>

                            <label
                                style="display: flex; align-items: center; gap: 8px; min-height: 32px; padding-bottom: 4px;"
                            >
                                <input
                                    type="checkbox"
                                    prop:checked=move || only_with_entries.get()
                                    on:change=move |ev| only_with_entries.set(event_target_checked(&ev))
                                />
                                <span>"Только с записями в GL"</span>
                            </label>

                            <label
                                style="display: flex; align-items: center; gap: 8px; min-height: 32px; padding-bottom: 4px;"
                            >
                                <input
                                    type="checkbox"
                                    prop:checked=move || only_journal_entries.get()
                                    on:change=move |ev| only_journal_entries.set(event_target_checked(&ev))
                                />
                                <span>"Только формирующие проводку"</span>
                            </label>
                        </Flex>
                    </div>
                </div>

                <div style="overflow: auto;">
                    <Table>
                        <TableHeader>
                            <TableRow>
                                <TableHeaderCell>"Code"</TableHeaderCell>
                                <TableHeaderCell>"Name"</TableHeaderCell>
                                <TableHeaderCell>"Classification"</TableHeaderCell>
                                <TableHeaderCell>"Rules"</TableHeaderCell>
                                <TableHeaderCell>"Journal"</TableHeaderCell>
                                <TableHeaderCell>"GL rows"</TableHeaderCell>
                                <TableHeaderCell>"Aliases / Sources"</TableHeaderCell>
                                <TableHeaderCell>"Info"</TableHeaderCell>
                            </TableRow>
                        </TableHeader>

                        <TableBody>
                            <Show
                                when=move || !filtered_items.get().is_empty()
                                fallback=move || view! {
                                    <TableRow>
                                        <TableCell attr:colspan="8">
                                            <TableCellLayout>
                                                {if loading.get() {
                                                    "Загрузка..."
                                                } else {
                                                    "Нет оборотов по текущему фильтру."
                                                }}
                                            </TableCellLayout>
                                        </TableCell>
                                    </TableRow>
                                }
                            >
                                <For
                                    each=move || filtered_items.get()
                                    key=|item| item.code.clone()
                                    children=move |item| {
                                        let journal_badge_color = if item.generates_journal_entry {
                                            BadgeColor::Success
                                        } else {
                                            BadgeColor::Warning
                                        };

                                        view! {
                                            <TableRow>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>
                                                        <code>{item.code.clone()}</code>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>{item.name.clone()}</div>
                                                        <div style="font-size: 12px; opacity: 0.75;">
                                                            {item.llm_description.clone()}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>{format!("group: {}", item.report_group.as_str())}</div>
                                                        <div>{format!("scope: {}", item.scope.as_str())}</div>
                                                        <div>{format!("value: {}", item.value_kind.as_str())}</div>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>{format!("agg: {}", item.agg_kind.as_str())}</div>
                                                        <div>{format!("select: {}", item.selection_rule.as_str())}</div>
                                                        <div>{format!("sign: {}", item.sign_policy.as_str())}</div>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>
                                                            <Badge
                                                                appearance=BadgeAppearance::Tint
                                                                color=journal_badge_color
                                                            >
                                                                {if item.generates_journal_entry {
                                                                    "yes"
                                                                } else {
                                                                    "no"
                                                                }}
                                                            </Badge>
                                                        </div>
                                                        <div style="margin-top: 6px;">
                                                            {format!(
                                                                "Дт {} / Кт {}",
                                                                if item.debit_account.is_empty() {
                                                                    "—"
                                                                } else {
                                                                    item.debit_account.as_str()
                                                                },
                                                                if item.credit_account.is_empty() {
                                                                    "—"
                                                                } else {
                                                                    item.credit_account.as_str()
                                                                }
                                                            )}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout attr:style="text-align: right;">
                                                        {item.gl_entries_count.to_string()}
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>
                                                            <strong>"Aliases: "</strong>
                                                            {join_or_dash(&item.aliases)}
                                                        </div>
                                                        <div style="margin-top: 6px;">
                                                            <strong>"Sources: "</strong>
                                                            {join_or_dash(&item.source_examples)}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>

                                                <TableCell>
                                                    <TableCellLayout>
                                                        <div>{item.description.clone()}</div>
                                                        <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                                                            {format!("Formula: {}", if item.formula_hint.is_empty() {
                                                                "—"
                                                            } else {
                                                                item.formula_hint.as_str()
                                                            })}
                                                        </div>
                                                        <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                                                            {format!("Journal: {}", if item.journal_comment.is_empty() {
                                                                "—"
                                                            } else {
                                                                item.journal_comment.as_str()
                                                            })}
                                                        </div>
                                                        <div style="margin-top: 6px; font-size: 12px; opacity: 0.8;">
                                                            {format!("Notes: {}", if item.notes.is_empty() {
                                                                "—"
                                                            } else {
                                                                item.notes.as_str()
                                                            })}
                                                        </div>
                                                    </TableCellLayout>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }
                                />
                            </Show>
                        </TableBody>
                    </Table>
                </div>
            </div>
        </PageFrame>
    }
}
