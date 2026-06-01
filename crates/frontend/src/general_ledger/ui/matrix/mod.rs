//! Страница «Матрица Слой / Оборот».
//!
//! Колонки — слои учёта (каждый разбит на 2 подколонки: измерения и проводки),
//! строки — обороты. Заголовок двухстрочный: 1-я строка — слой, 2-я — подколонки
//! «изм»/«пров» с сортировкой. Popover раскрывает измерения 1-го уровня со
//! списком источников (где доступен разрез) и проекции-зеркала ячейки.
//!
//! Данные — из эндпоинта `/api/general-ledger/layer-turnover-matrix`, который
//! собирает их естественно из реестров (слои/обороты/измерения) + overlay
//! счётчиков из GL. Фильтрация и сортировка — целиком на клиенте.

use std::collections::HashMap;

use contracts::general_ledger::{GlMatrixCell, GlMatrixLayer, GlMatrixTurnover};
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

use crate::general_ledger::api::fetch_gl_layer_turnover_matrix;
use crate::general_ledger::ui::dimension_chip::{chip_from_dimension, GlDimensionChip};
use crate::general_ledger::ui::layer_badge::GlLayerBadge;
use crate::shared::modal_frame::ModalFrame;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_LIST;

/// Метрика подколонки слоя.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SortMetric {
    /// Число измерений 1-го уровня.
    Dim,
    /// Число GL-проводок.
    Qty,
}

/// Ключ сортировки колонки.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SortKey {
    /// Первая колонка — наименование оборота.
    Turnover,
    /// Подколонка слоя: (код слоя, метрика).
    Cell(String, SortMetric),
}

fn cell_key(turnover_code: &str, layer: &str) -> (String, String) {
    (turnover_code.to_string(), layer.to_string())
}

fn cell_has_filter(cell: &GlMatrixCell, filter: &str) -> bool {
    filter.is_empty() || cell.dimensions.iter().any(|dim| dim.def.id == filter)
}

/// Целое с неразрывным пробелом-разделителем тысяч.
fn fmt_int(value: i64) -> String {
    let negative = value < 0;
    let digits = value.unsigned_abs().to_string();
    let bytes = digits.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len + len / 3);
    for (index, byte) in bytes.iter().enumerate() {
        if index > 0 && (len - index) % 3 == 0 {
            out.push('\u{00A0}');
        }
        out.push(*byte as char);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

#[component]
pub fn GeneralLedgerLayerTurnoverMatrixPage() -> impl IntoView {
    let layers = RwSignal::new(Vec::<GlMatrixLayer>::new());
    let turnovers = RwSignal::new(Vec::<GlMatrixTurnover>::new());
    let cells = RwSignal::new(HashMap::<(String, String), GlMatrixCell>::new());
    let filter_dimensions = RwSignal::new(Vec::<contracts::general_ledger::GlDimensionDef>::new());

    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let loaded = RwSignal::new(false);

    // "" — фильтр выключен; иначе id измерения.
    let dim_filter = RwSignal::new(String::new());
    // Открытая ячейка для popover.
    let modal_cell = RwSignal::new(None::<GlMatrixCell>);
    // Сортировка: (ключ колонки, desc). None — порядок реестра.
    let sort = RwSignal::new(None::<(SortKey, bool)>);

    let load = move || {
        spawn_local(async move {
            loading.set(true);
            error.set(None);
            match fetch_gl_layer_turnover_matrix().await {
                Ok(response) => {
                    layers.set(response.layers);
                    turnovers.set(response.turnovers);
                    filter_dimensions.set(response.filter_dimensions);
                    let map = response
                        .cells
                        .into_iter()
                        .map(|cell| (cell_key(&cell.turnover_code, &cell.layer), cell))
                        .collect::<HashMap<_, _>>();
                    cells.set(map);
                    loaded.set(true);
                }
                Err(err) => error.set(Some(err)),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        if !loaded.get() {
            load();
        }
    });

    // Клик по заголовку: тот же ключ — переключаем направление, иначе новый ключ
    // по убыванию.
    let toggle_sort = move |key: SortKey| {
        sort.update(|current| match current {
            Some((k, desc)) if *k == key => *desc = !*desc,
            _ => *current = Some((key, true)),
        });
    };

    // Строки с учётом фильтра и сортировки.
    let visible_turnovers = Signal::derive(move || {
        let filter = dim_filter.get();
        let map = cells.get();
        let layer_list = layers.get();
        let mut list = turnovers.get();

        if !filter.is_empty() {
            list.retain(|turnover| {
                layer_list.iter().any(|layer| {
                    map.get(&cell_key(&turnover.code, &layer.code))
                        .map(|cell| cell_has_filter(cell, &filter))
                        .unwrap_or(false)
                })
            });
        }

        if let Some((key, desc)) = sort.get() {
            match &key {
                SortKey::Turnover => list.sort_by(|a, b| a.name.cmp(&b.name)),
                SortKey::Cell(layer, metric) => {
                    let value = |code: &str| -> i64 {
                        map.get(&cell_key(code, layer))
                            .map(|cell| match metric {
                                SortMetric::Dim => cell.top_level_count as i64,
                                SortMetric::Qty => cell.entry_count,
                            })
                            .unwrap_or(-1)
                    };
                    list.sort_by(|a, b| value(&a.code).cmp(&value(&b.code)));
                }
            }
            if desc {
                list.reverse();
            }
        }

        list
    });

    let row_count = Signal::derive(move || visible_turnovers.get().len());
    let total_count = Signal::derive(move || turnovers.get().len());

    view! {
        <PageFrame page_id="general_ledger_matrix--grid" category=PAGE_CAT_LIST class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"Матрица Слой / Оборот"</h1>
                    <Badge appearance=BadgeAppearance::Tint color=BadgeColor::Brand>
                        {move || row_count.get().to_string()}
                    </Badge>
                    <span style="font-size: 12px; opacity: 0.75;">
                        {move || format!("из {} оборотов", total_count.get())}
                    </span>
                </div>

                <div class="page__header-right">
                    <span style="font-size: 12px; opacity: 0.8;">"Фильтр по измерению:"</span>
                    <Select value=dim_filter>
                        <option value="">"— все измерения —"</option>
                        {move || filter_dimensions.get().into_iter().map(|dim| {
                            let value = dim.id.clone();
                            let label = format!("{} ({})", dim.label, dim.code);
                            view! { <option value=value>{label}</option> }
                        }).collect_view()}
                    </Select>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| { dim_filter.set(String::new()); sort.set(None); }
                    >
                        "Сбросить"
                    </Button>
                    <Button
                        appearance=ButtonAppearance::Secondary
                        on_click=move |_| load()
                        disabled=Signal::derive(move || loading.get())
                    >
                        {move || if loading.get() { "Загрузка..." } else { "Обновить" }}
                    </Button>
                </div>
            </div>

            <div class="page__content gl-matrix-page">
                {move || error.get().map(|err| view! {
                    <div class="alert alert--error">{err}</div>
                })}

                <div class="gl-matrix__hint">
                    "Колонки слоёв разбиты на «изм» (число измерений 1-го уровня) и «пров» "
                    "(число GL-проводок). Бледная ячейка — комбинация существует только "
                    "теоретически (0 проводок). Клик по подзаголовку — сортировка."
                </div>

                <div class="table-wrapper gl-matrix__wrapper">
                    <table class="table gl-matrix__table">
                        <thead>
                            {move || {
                                let active = sort.get();
                                let arrow = move |key: &SortKey| -> &'static str {
                                    match &active {
                                        Some((k, desc)) if k == key => {
                                            if *desc { " ▼" } else { " ▲" }
                                        }
                                        _ => "",
                                    }
                                };
                                let layer_list = layers.get();
                                let layer_list_2 = layer_list.clone();
                                view! {
                                    <tr>
                                        <th
                                            rowspan=2
                                            class="table__cell gl-matrix__corner gl-matrix__sortable"
                                            on:click=move |_| toggle_sort(SortKey::Turnover)
                                        >
                                            {format!("Оборот \\ Слой{}", arrow(&SortKey::Turnover))}
                                        </th>
                                        {layer_list.into_iter().map(|layer| {
                                            view! {
                                                <th colspan=2 class="table__cell table__cell--center gl-matrix__layerhead">
                                                    <GlLayerBadge layer=layer.code.clone() />
                                                    <div style="font-size:11px; opacity:0.7;">{layer.name.clone()}</div>
                                                </th>
                                            }
                                        }).collect_view()}
                                    </tr>
                                    <tr>
                                        {layer_list_2.into_iter().map(|layer| {
                                            let dim_key = SortKey::Cell(layer.code.clone(), SortMetric::Dim);
                                            let qty_key = SortKey::Cell(layer.code.clone(), SortMetric::Qty);
                                            let dim_arrow = arrow(&dim_key).to_string();
                                            let qty_arrow = arrow(&qty_key).to_string();
                                            let dim_click = dim_key.clone();
                                            let qty_click = qty_key.clone();
                                            view! {
                                                <th
                                                    class="table__cell table__cell--center gl-matrix__subhead gl-matrix__group-start gl-matrix__sortable"
                                                    title="Сортировать по числу измерений"
                                                    on:click=move |_| toggle_sort(dim_click.clone())
                                                >
                                                    {format!("изм{dim_arrow}")}
                                                </th>
                                                <th
                                                    class="table__cell table__cell--right gl-matrix__subhead gl-matrix__sortable"
                                                    title="Сортировать по числу проводок"
                                                    on:click=move |_| toggle_sort(qty_click.clone())
                                                >
                                                    {format!("пров{qty_arrow}")}
                                                </th>
                                            }
                                        }).collect_view()}
                                    </tr>
                                }
                            }}
                        </thead>
                        <tbody>
                            {move || {
                                let filter = dim_filter.get();
                                let layer_list = layers.get();
                                let map = cells.get();
                                visible_turnovers.get().into_iter().map(|turnover| {
                                    let layer_list = layer_list.clone();
                                    let map = map.clone();
                                    let filter = filter.clone();
                                    let turnover_code = turnover.code.clone();
                                    view! {
                                        <tr class="table__row">
                                            <td class="table__cell gl-matrix__rowhead">
                                                <span class="gl-matrix__turnover-name">{turnover.name.clone()}</span>
                                                <code class="text-code gl-matrix__turnover-code">{turnover.code.clone()}</code>
                                            </td>
                                            {layer_list.into_iter().map(|layer| {
                                                let cell = map.get(&cell_key(&turnover_code, &layer.code)).cloned();
                                                render_cell(cell, &filter, modal_cell)
                                            }).collect_view()}
                                        </tr>
                                    }
                                }).collect_view()
                            }}
                        </tbody>
                    </table>
                </div>
            </div>

            // ── Popover ячейки ──────────────────────────────────────────────────
            {move || {
                let Some(cell) = modal_cell.get() else {
                    return view! { <></> }.into_any();
                };
                render_cell_modal(cell, modal_cell).into_any()
            }}
        </PageFrame>
    }
}

/// Рендер пары подколонок одного слоя: «изм» (по центру) и «пров» (справа).
fn render_cell(
    cell: Option<GlMatrixCell>,
    filter: &str,
    modal_cell: RwSignal<Option<GlMatrixCell>>,
) -> impl IntoView {
    let Some(cell) = cell else {
        return view! {
            <td class="table__cell table__cell--center gl-matrix__cell gl-matrix__cell--dim gl-matrix__group-start gl-matrix__cell--empty">"—"</td>
            <td class="table__cell table__cell--right gl-matrix__cell gl-matrix__cell--qty gl-matrix__cell--empty">"—"</td>
        }.into_any();
    };

    if !cell_has_filter(&cell, filter) {
        // Ячейка существует, но не содержит выбранного измерения — приглушаем.
        return view! {
            <td class="table__cell table__cell--center gl-matrix__cell gl-matrix__cell--dim gl-matrix__group-start gl-matrix__cell--muted">"·"</td>
            <td class="table__cell table__cell--right gl-matrix__cell gl-matrix__cell--qty gl-matrix__cell--muted">"·"</td>
        }.into_any();
    }

    let theoretical = if cell.entry_count == 0 {
        " gl-matrix__cell--theoretical"
    } else {
        ""
    };
    let count = cell.top_level_count;
    let qty = fmt_int(cell.entry_count);
    let cell_for_click = cell.clone();

    view! {
        <td class=format!("table__cell table__cell--center gl-matrix__cell gl-matrix__cell--dim gl-matrix__group-start{theoretical}")>
            <button
                type="button"
                class="gl-matrix__dim-btn"
                title="Измерения 1-го уровня — открыть детали"
                on:click=move |_| modal_cell.set(Some(cell_for_click.clone()))
            >
                {count}
            </button>
        </td>
        <td class=format!("table__cell table__cell--right gl-matrix__cell gl-matrix__cell--qty{theoretical}")>
            <span class="gl-matrix__qty">{qty}</span>
        </td>
    }.into_any()
}

/// Popover с подробностями ячейки: измерения 1-го уровня (db_field, источник) и
/// проекции-зеркала. Вложенные номенклатурные разрезы (dim1–dim6) не показываем —
/// они одинаковы и ведут себя так же, как корневое измерение «Номенклатура».
fn render_cell_modal(
    cell: GlMatrixCell,
    modal_cell: RwSignal<Option<GlMatrixCell>>,
) -> impl IntoView {
    let turnover_code = cell.turnover_code.clone();
    let layer = cell.layer.clone();
    let entry_count = cell.entry_count;
    let dimensions = cell
        .dimensions
        .iter()
        .filter(|dim| dim.is_top_level)
        .cloned()
        .collect::<Vec<_>>();
    let projections = cell.projections.clone();

    view! {
        <ModalFrame
            on_close=Callback::new(move |_| modal_cell.set(None))
            modal_style="width: 520px; padding: 0; overflow: hidden;".to_string()
        >
            <div style="padding: 14px 18px 12px; border-bottom: 1px solid var(--color-border);">
                <div style="display:flex; align-items:center; gap:8px;">
                    <GlLayerBadge layer=layer.clone() />
                    <span style="font-size: 14px; font-weight: 600;">{turnover_code.clone()}</span>
                </div>
                <div style="font-size: 12px; color: var(--color-text-secondary); margin-top: 4px;">
                    {format!("GL-проводок: {}", fmt_int(entry_count))}
                </div>
            </div>

            <div style="padding: 12px 18px; max-height: 60vh; overflow-y: auto;">
                <div style="font-size:12px; font-weight:600; margin-bottom:6px;">
                    {format!("Измерения 1-го уровня ({})", dimensions.len())}
                </div>
                <div class="gl-matrix__dim-list">
                    {dimensions.into_iter().map(|dim| {
                        let chip = chip_from_dimension(&dim.def);
                        view! {
                            <div class="gl-matrix__dim-row">
                                <GlDimensionChip
                                    label=chip.label
                                    color_key=chip.color_key
                                    title=chip.title
                                />
                                <span class="gl-matrix__dim-label">{dim.def.label.clone()}</span>
                                <div class="gl-matrix__dim-sources" title="Где доступен этот разрез">
                                    {dim.sources.iter().map(|src| view! {
                                        <span class="gl-matrix__src">{src.clone()}</span>
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                    }).collect_view()}
                </div>

                <div style="font-size:12px; font-weight:600; margin:14px 0 6px;">
                    {format!("Проекции-зеркала ({})", projections.len())}
                </div>
                <div class="gl-matrix__proj-list">
                    {projections.into_iter().map(|proj| {
                        view! {
                            <div class="gl-matrix__proj-row">
                                <code class="text-code">{proj.resource_table.clone()}</code>
                                <span class="gl-matrix__proj-kind">{proj.kind.clone()}</span>
                            </div>
                        }
                    }).collect_view()}
                </div>
            </div>
        </ModalFrame>
    }
}
