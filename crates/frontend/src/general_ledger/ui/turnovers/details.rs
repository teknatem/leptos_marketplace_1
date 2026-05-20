use crate::general_ledger::api::fetch_general_ledger_turnovers;
use crate::general_ledger::ui::dimensions::DimensionPreview;
use crate::layout::global_context::AppGlobalContext;
use crate::layout::tabs::tab_label_for_key;
use crate::shared::clipboard::copy_to_clipboard;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::general_ledger::GeneralLedgerTurnoverDto;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

fn val(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}

fn list_val(values: &[String]) -> String {
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

/// Одна строка ключ–значение внутри секции карточки.
#[component]
fn KvRow(label: &'static str, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="gl-td-row">
            <span class="gl-td-key">{label}</span>
            <span class="gl-td-val">{value}</span>
        </div>
    }
}

/// Одна строка ключ–значение где значение рендерится произвольно.
#[component]
fn KvRowSlot(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="gl-td-row">
            <span class="gl-td-key">{label}</span>
            <span class="gl-td-val">{children()}</span>
        </div>
    }
}

#[component]
pub fn GeneralLedgerTurnoverDetails(code: String, on_close: Callback<()>) -> impl IntoView {
    let tabs_store =
        leptos::context::use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    let (item, set_item) = signal::<Option<GeneralLedgerTurnoverDto>>(None);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal::<Option<String>>(None);

    let code_clone = code.clone();
    Effect::new(move |_| {
        let code = code_clone.clone();
        spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);
            match fetch_general_ledger_turnovers().await {
                Ok(response) => {
                    let found = response.items.into_iter().find(|t| t.code == code);
                    if found.is_none() {
                        set_error.set(Some(format!("Оборот «{}» не найден", code)));
                    }
                    set_item.set(found);
                }
                Err(err) => set_error.set(Some(err)),
            }
            set_loading.set(false);
        });
    });

    view! {
        <PageFrame page_id="general_ledger_turnover_details" category=PAGE_CAT_DETAIL>
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || item.get()
                            .map(|t| t.name.clone())
                            .unwrap_or_else(|| "Оборот GL".to_string())}
                    </h1>
                    {move || item.get().map(|t| {
                        let color = if t.gl_entries_count > 0 { BadgeColor::Success } else { BadgeColor::Subtle };
                        view! {
                            <Badge appearance=BadgeAppearance::Tint color=color>
                                {format!("{} записей GL", t.gl_entries_count)}
                            </Badge>
                        }
                    })}
                </div>

                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary
                        on_click=move |_| tabs_store.open_tab("general_ledger_turnovers", "Обороты GL")>
                        "Все обороты"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary
                        on_click=move |_| tabs_store.open_tab(
                            &format!("general_ledger_dimensions__{}", code),
                            tab_label_for_key("general_ledger_dimensions"),
                        )>
                        "Измерения GL"
                    </Button>
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || error.get().map(|err| view! { <div class="alert alert--error">{err}</div> })}

                {move || if loading.get() && item.get().is_none() {
                    view! { <div class="page__placeholder">"Загрузка..."</div> }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}

                {move || item.get().map(|t| {
                    let sig = t.dimension_signature.clone();
                    let sig_copy = sig.clone();
                    let sig_empty = sig.trim().is_empty();
                    let tc_for_dims = t.code.clone();
                    let journal_color = if t.generates_journal_entry { BadgeColor::Success } else { BadgeColor::Warning };

                    view! {
                        <div class="gl-td-page">

                            // ── Hero ───────────────────────────────────────────────────
                            <div class="gl-td-hero">
                                <code class="gl-td-hero__code">{t.code.clone()}</code>
                                <p class="gl-td-hero__desc">
                                    {val(&t.description).to_string()}
                                </p>
                                {if !t.llm_description.is_empty() {
                                    view! {
                                        <p class="gl-td-hero__llm">{t.llm_description.clone()}</p>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }}
                            </div>

                            // ── Две колонки: Классификация | Проводка ──────────────────
                            <div class="gl-td-cols">
                                <div class="gl-td-section">
                                    <div class="gl-td-section__title">"Классификация"</div>
                                    <KvRow label="Group" value=t.report_group.as_str().to_string() />
                                    <KvRow label="Scope" value=t.scope.as_str().to_string() />
                                    <KvRow label="Value" value=t.value_kind.as_str().to_string() />
                                    <KvRow label="Agg" value=t.agg_kind.as_str().to_string() />
                                    <KvRow label="Selection" value=t.selection_rule.as_str().to_string() />
                                    <KvRow label="Sign" value=t.sign_policy.as_str().to_string() />
                                </div>

                                <div class="gl-td-section">
                                    <div class="gl-td-section__title">"Журнальная проводка"</div>
                                    <KvRowSlot label="Проводка">
                                        <Badge appearance=BadgeAppearance::Tint color=journal_color>
                                            {if t.generates_journal_entry { "Формирует" } else { "Не формирует" }}
                                        </Badge>
                                    </KvRowSlot>
                                    <KvRow label="Дебет" value=val(&t.debit_account).to_string() />
                                    <KvRow label="Кредит" value=val(&t.credit_account).to_string() />
                                    {if !t.journal_comment.is_empty() {
                                        view! {
                                            <KvRow label="Комментарий" value=t.journal_comment.clone() />
                                        }.into_any()
                                    } else {
                                        view! { <></> }.into_any()
                                    }}
                                </div>
                            </div>

                            // ── Источники ──────────────────────────────────────────────
                            <div class="gl-td-section">
                                <div class="gl-td-section__title">"Источники данных"</div>
                                {if !t.formula_hint.is_empty() {
                                    view! { <KvRow label="Formula" value=t.formula_hint.clone() /> }.into_any()
                                } else { view! { <></> }.into_any() }}
                                <KvRow label="Sources" value=list_val(&t.source_examples) />
                                <KvRow label="Aliases" value=list_val(&t.aliases) />
                                {if !t.notes.is_empty() {
                                    view! { <KvRow label="Notes" value=t.notes.clone() /> }.into_any()
                                } else { view! { <></> }.into_any() }}
                            </div>

                            // ── Измерения ──────────────────────────────────────────────
                            <div class="gl-td-section">
                                <div class="gl-td-section__title">
                                    "Измерения"
                                    <button
                                        type="button"
                                        class="gl-td-section__link"
                                        on:click=move |_| tabs_store.open_tab(
                                            &format!("general_ledger_dimensions__{}", tc_for_dims),
                                            tab_label_for_key("general_ledger_dimensions"),
                                        )
                                    >
                                        "Открыть каталог →"
                                    </button>
                                </div>
                                <KvRowSlot label="Signature">
                                    <code class="gl-td-mono">
                                        {if sig_empty { "—".to_string() } else { sig.clone() }}
                                    </code>
                                    {if !sig_empty {
                                        view! {
                                            <button
                                                type="button"
                                                class="gldim-copy-btn"
                                                title="Copy signature"
                                                on:click=move |_| copy_to_clipboard(&sig_copy)
                                            >"⎘"</button>
                                        }.into_any()
                                    } else { view! { <></> }.into_any() }}
                                </KvRowSlot>
                                {if !t.available_dimensions.is_empty() {
                                    view! {
                                        <KvRowSlot label="Preview">
                                            <DimensionPreview dimensions=t.available_dimensions.clone() />
                                        </KvRowSlot>
                                    }.into_any()
                                } else { view! { <></> }.into_any() }}
                            </div>

                        </div>
                    }.into_any()
                })}
            </div>
        </PageFrame>
    }
}
