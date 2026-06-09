use crate::general_ledger::ui::entity_badge::GlEntityBadge;
use crate::general_ledger::ui::layer_badge::GlLayerBadge;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::clipboard::copy_to_clipboard_with_callback;
use crate::shared::components::card_animated::CardAnimated;
use crate::shared::date_utils::{format_date, format_datetime, format_datetime_space};
use crate::shared::icons::icon;
use crate::shared::list_utils::{get_sort_class, get_sort_indicator};
use contracts::general_ledger::GeneralLedgerEntryDto;
use leptos::prelude::*;
use thaw::*;

pub const DOCUMENT_GENERAL_LEDGER_ENTRIES_NAV_SUFFIX: &str = "general_ledger_entries_table";

pub fn document_general_ledger_entries_nav_id(document_key: &str) -> String {
    format!("{document_key}_details_{DOCUMENT_GENERAL_LEDGER_ENTRIES_NAV_SUFFIX}")
}

fn short_id(value: &str) -> &str {
    if value.len() >= 8 {
        &value[..8]
    } else {
        value
    }
}

fn fmt_amount(value: f64) -> String {
    format!("{value:.2}")
}

fn fmt_amount_excel(value: f64) -> String {
    fmt_amount(value).replace('.', ",")
}

fn fmt_date_time(value: &str) -> String {
    if value.contains('T') {
        format_datetime(value)
    } else if value.contains(' ') {
        format_datetime_space(value)
    } else {
        format_date(value)
    }
}

fn turnover_name(entry: &GeneralLedgerEntryDto) -> &str {
    if entry.turnover_name.trim().is_empty() {
        entry.turnover_code.as_str()
    } else {
        entry.turnover_name.as_str()
    }
}

fn sort_entries(rows: &mut [GeneralLedgerEntryDto], field: &str, ascending: bool) {
    rows.sort_by(|left, right| {
        let ord = match field {
            "entry_date" => left.entry_date.cmp(&right.entry_date),
            "layer" => left.layer.as_str().cmp(right.layer.as_str()),
            "entity" => left
                .entity
                .as_deref()
                .unwrap_or("")
                .cmp(right.entity.as_deref().unwrap_or("")),
            "turnover_name" => turnover_name(left).cmp(turnover_name(right)),
            "turnover_code" => left.turnover_code.cmp(&right.turnover_code),
            "debit_account" => left.debit_account.cmp(&right.debit_account),
            "credit_account" => left.credit_account.cmp(&right.credit_account),
            "amount" => left
                .amount
                .partial_cmp(&right.amount)
                .unwrap_or(std::cmp::Ordering::Equal),
            "id" => left.id.cmp(&right.id),
            _ => left.entry_date.cmp(&right.entry_date),
        };
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
}

fn tsv_cell(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .replace('\t', " ")
        .replace('\r', " ")
        .replace('\n', " ")
}

fn build_excel_tsv(rows: &[GeneralLedgerEntryDto]) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(
        [
            "Дата",
            "Слой",
            "Субъект",
            "Наименование оборота",
            "Код оборота",
            "Дт",
            "Кт",
            "Сумма",
            "ID",
        ]
        .join("\t"),
    );

    for entry in rows {
        lines.push(
            [
                tsv_cell(fmt_date_time(&entry.entry_date)),
                tsv_cell(entry.layer.as_str()),
                tsv_cell(entry.entity.as_deref().unwrap_or("")),
                tsv_cell(turnover_name(entry)),
                tsv_cell(&entry.turnover_code),
                tsv_cell(&entry.debit_account),
                tsv_cell(&entry.credit_account),
                tsv_cell(fmt_amount_excel(entry.amount)),
                tsv_cell(&entry.id),
            ]
            .join("\t"),
        );
    }

    lines.join("\n")
}

#[component]
pub fn DocumentGeneralLedgerEntries(
    entries: Signal<Vec<GeneralLedgerEntryDto>>,
    loading: Signal<bool>,
    error: Signal<Option<String>>,
    nav_id: String,
    title: &'static str,
    empty_message: &'static str,
) -> impl IntoView {
    let tabs = use_context::<AppGlobalContext>();
    let nav_id = StoredValue::new(nav_id);
    let copied = RwSignal::new(false);
    let sort_field = RwSignal::new("entry_date".to_string());
    let sort_ascending = RwSignal::new(true);
    let toggle_sort = move |field: &'static str| {
        if sort_field.get_untracked() == field {
            sort_ascending.update(|value| *value = !*value);
        } else {
            sort_field.set(field.to_string());
            sort_ascending.set(true);
        }
    };
    let open_entry = move |entry_id: String| {
        if let Some(tabs) = tabs.as_ref() {
            tabs.open_tab(
                &format!("general_ledger_details_{entry_id}"),
                &format!("General Ledger {}", short_id(&entry_id)),
            );
        }
    };

    view! {
        {move || {
            if loading.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id=nav_id.get_value()>
                        <Flex gap=FlexGap::Small style="align-items: center; justify-content: center; padding: var(--spacing-xl);">
                            <Spinner />
                            <span>"Загрузка записей General Ledger..."</span>
                        </Flex>
                    </CardAnimated>
                }.into_any();
            }

            if let Some(err) = error.get() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id=nav_id.get_value()>
                        <h4 class="details-section__title">{title}</h4>
                        <div class="alert alert--error">{err}</div>
                    </CardAnimated>
                }.into_any();
            }

            let mut rows = entries.get();
            if rows.is_empty() {
                return view! {
                    <CardAnimated delay_ms=0 nav_id=nav_id.get_value()>
                        <h4 class="details-section__title">{title}</h4>
                        <div class="text-muted">{empty_message}</div>
                    </CardAnimated>
                }.into_any();
            }

            let row_count = rows.len();
            let total_amount: f64 = rows.iter().map(|entry| entry.amount).sum();
            let posting_id = rows.first().map(|entry| entry.id.clone()).unwrap_or_default();
            let current_sort_field = sort_field.get();
            sort_entries(&mut rows, current_sort_field.as_str(), sort_ascending.get());
            let copy_text = build_excel_tsv(&rows);

            view! {
                <CardAnimated delay_ms=0 nav_id=nav_id.get_value()>
                    <h4 class="details-section__title">{title}</h4>
                    <div style="display:flex;gap:12px;align-items:center;justify-content:space-between;flex-wrap:wrap;margin-bottom:var(--spacing-md);">
                        <div style="display:flex;gap:12px;flex-wrap:wrap;align-items:center;">
                            <span class="badge badge--primary">{format!("Проводок: {row_count}")}</span>
                            <span class="badge badge--neutral">{format!("Проведение: {}", short_id(&posting_id))}</span>
                            <span class="badge badge--success">{format!("Итого: {}", fmt_amount(total_amount))}</span>
                        </div>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            size=ButtonSize::Small
                            on_click=move |_| {
                                copied.set(false);
                                let copied = copied;
                                copy_to_clipboard_with_callback(&copy_text, move || copied.set(true));
                            }
                        >
                            {icon("copy")}
                            {move || if copied.get() { " Скопировано" } else { " Копировать в Excel" }}
                        </Button>
                    </div>

                    <div class="table-wrapper" style="max-width:1180px;margin-left:auto;margin-right:auto;">
                        <Table attr:style="width:100%;min-width:1040px;table-layout:fixed;">
                            <TableHeader>
                                <TableRow>
                                    <TableHeaderCell min_width=120.0 attr:style="width:120px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("entry_date")>
                                            "Дата"
                                            <span class=move || get_sort_class(&sort_field.get(), "entry_date")>
                                                {move || get_sort_indicator(&sort_field.get(), "entry_date", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=72.0 attr:style="width:72px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("layer")>
                                            "Слой"
                                            <span class=move || get_sort_class(&sort_field.get(), "layer")>
                                                {move || get_sort_indicator(&sort_field.get(), "layer", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=76.0 attr:style="width:76px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("entity")>
                                            "Субъект"
                                            <span class=move || get_sort_class(&sort_field.get(), "entity")>
                                                {move || get_sort_indicator(&sort_field.get(), "entity", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=280.0 attr:style="width:auto;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("turnover_name")>
                                            "Наименование оборота"
                                            <span class=move || get_sort_class(&sort_field.get(), "turnover_name")>
                                                {move || get_sort_indicator(&sort_field.get(), "turnover_name", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=170.0 attr:style="width:170px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("turnover_code")>
                                            "Код оборота"
                                            <span class=move || get_sort_class(&sort_field.get(), "turnover_code")>
                                                {move || get_sort_indicator(&sort_field.get(), "turnover_code", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=76.0 attr:style="width:76px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("debit_account")>
                                            "Дт"
                                            <span class=move || get_sort_class(&sort_field.get(), "debit_account")>
                                                {move || get_sort_indicator(&sort_field.get(), "debit_account", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=76.0 attr:style="width:76px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("credit_account")>
                                            "Кт"
                                            <span class=move || get_sort_class(&sort_field.get(), "credit_account")>
                                                {move || get_sort_indicator(&sort_field.get(), "credit_account", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=118.0 attr:style="width:118px;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("amount")>
                                            "Сумма"
                                            <span class=move || get_sort_class(&sort_field.get(), "amount")>
                                                {move || get_sort_indicator(&sort_field.get(), "amount", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                    <TableHeaderCell min_width=120.0 attr:style="width:120px;border-right:0;">
                                        <div class="table__sortable-header" style="cursor:pointer;" on:click=move |_| toggle_sort("id")>
                                            "ID"
                                            <span class=move || get_sort_class(&sort_field.get(), "id")>
                                                {move || get_sort_indicator(&sort_field.get(), "id", sort_ascending.get())}
                                            </span>
                                        </div>
                                    </TableHeaderCell>
                                </TableRow>
                            </TableHeader>
                            <TableBody>
                                <For
                                    each=move || rows.clone()
                                    key=|entry| entry.id.clone()
                                    children=move |entry| {
                                        let entry_id = entry.id.clone();
                                        let entry_id_for_click = entry.id.clone();
                                        let turnover_name = if entry.turnover_name.trim().is_empty() {
                                            entry.turnover_code.clone()
                                        } else {
                                            entry.turnover_name.clone()
                                        };
                                        view! {
                                            <TableRow>
                                                <TableCell attr:style="width:120px;">
                                                    <TableCellLayout>{fmt_date_time(&entry.entry_date)}</TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:72px;">
                                                    <TableCellLayout>
                                                        <GlLayerBadge layer=entry.layer.as_str().to_string() />
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:76px;">
                                                    <TableCellLayout>
                                                        {match entry.entity.as_deref() {
                                                            Some(code) if !code.is_empty() => {
                                                                view! { <GlEntityBadge entity=code.to_string() /> }.into_any()
                                                            }
                                                            _ => view! { <span class="text-muted">"—"</span> }.into_any(),
                                                        }}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell>
                                                    <TableCellLayout truncate=true>{turnover_name}</TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:170px;">
                                                    <TableCellLayout truncate=true>{entry.turnover_code.clone()}</TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:76px;">
                                                    <TableCellLayout>
                                                        <span class="badge badge--success">{entry.debit_account.clone()}</span>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:76px;">
                                                    <TableCellLayout>
                                                        <span class="badge badge--primary">{entry.credit_account.clone()}</span>
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell class="text-right" attr:style="width:118px;text-align:right;">
                                                    <TableCellLayout attr:style="display:block;width:100%;text-align:right;">
                                                        {fmt_amount(entry.amount)}
                                                    </TableCellLayout>
                                                </TableCell>
                                                <TableCell attr:style="width:120px;border-right:0;">
                                                    <TableCellLayout>
                                                        <span
                                                            class="table__link"
                                                            style="display:inline-flex;align-items:center;gap:4px;white-space:nowrap;"
                                                            on:click=move |_| open_entry(entry_id_for_click.clone())
                                                        >
                                                            {short_id(&entry_id).to_string()}
                                                            {icon("external-link")}
                                                        </span>
                                                    </TableCellLayout>
                                                </TableCell>
                                            </TableRow>
                                        }
                                    }
                                />
                            </TableBody>
                        </Table>
                    </div>
                </CardAnimated>
            }.into_any()
        }}
    }
}
