//! Детали документа поисковой аналитики WB (a040): общая сводка + строки по товарам
//! (показы/переходы/CTR/позиция/заказы) + топ поисковых запросов на товар.

use crate::shared::api_utils::api_base;
use crate::shared::page_frame::PageFrame;
use crate::shared::page_standard::PAGE_CAT_DETAIL;
use contracts::domain::a040_wb_search_analytics_daily::aggregate::WbSearchAnalyticsDailyLine;
use gloo_net::http::Request;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use thaw::*;

#[derive(Debug, Clone, Deserialize)]
struct DetailsDto {
    #[allow(dead_code)]
    id: String,
    document_no: String,
    document_date: String,
    connection_id: String,
    #[serde(default)]
    total_open_card: i64,
    #[serde(default)]
    total_orders: i64,
    #[serde(default)]
    fetched_at: String,
    #[serde(default)]
    lines: Vec<WbSearchAnalyticsDailyLine>,
}

#[component]
pub fn WbSearchAnalyticsDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (doc, set_doc) = signal::<Option<DetailsDto>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);

    {
        let id = id.clone();
        spawn_local(async move {
            let url = format!("{}/api/a040/wb-search-analytics/{}", api_base(), id);
            match Request::get(&url).send().await {
                Ok(resp) if resp.ok() => match resp.json::<DetailsDto>().await {
                    Ok(d) => set_doc.set(Some(d)),
                    Err(e) => set_error.set(Some(format!("Ошибка парсинга: {}", e))),
                },
                Ok(resp) => set_error.set(Some(format!("Ошибка сервера: {}", resp.status()))),
                Err(e) => set_error.set(Some(format!("Ошибка сети: {}", e))),
            }
            set_loading.set(false);
        });
    }

    view! {
        <PageFrame page_id="a040_wb_search_analytics_daily--detail" category=PAGE_CAT_DETAIL class="page--wide">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">
                        {move || doc.get().map(|d| format!("Поиск WB {}", d.document_date)).unwrap_or_else(|| "Поисковая аналитика WB".to_string())}
                    </h1>
                </div>
                <div class="page__header-right">
                    <Button appearance=ButtonAppearance::Secondary on_click=move |_| on_close.run(())>
                        "Закрыть"
                    </Button>
                </div>
            </div>

            <div class="page__content">
                {move || {
                    if loading.get() {
                        view! {
                            <Flex gap=FlexGap::Small style="align-items:center;justify-content:center;padding:var(--spacing-4xl);">
                                <Spinner />
                                <span>"Загрузка..."</span>
                            </Flex>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! { <div class="alert alert--error">{err}</div> }.into_any()
                    } else if let Some(d) = doc.get() {
                        let lines = d.lines.clone();
                        view! {
                            <Card>
                                <Flex gap=FlexGap::Large>
                                    <div><b>"Документ: "</b>{d.document_no.clone()}</div>
                                    <div><b>"Дата: "</b>{d.document_date.clone()}</div>
                                    <div><b>"Кабинет: "</b>{d.connection_id.clone()}</div>
                                    <div><b>"Показы: "</b>"не предоставляются WB"</div>
                                    <div><b>"Переходы: "</b>{d.total_open_card}</div>
                                    <div><b>"Заказы из поиска: "</b>{d.total_orders}</div>
                                    <div><b>"Загружено: "</b>{d.fetched_at.clone()}</div>
                                </Flex>
                            </Card>

                            <div class="table-wrapper" style="margin-top:16px;">
                                <Table attr:style="width:100%;min-width:1000px;">
                                    <TableHeader>
                                        <TableRow>
                                            <TableHeaderCell>"nmID"</TableHeaderCell>
                                            <TableHeaderCell>"Товар"</TableHeaderCell>
                                            <TableHeaderCell>"Бренд"</TableHeaderCell>
                                            <TableHeaderCell>"Видимость"</TableHeaderCell>
                                            <TableHeaderCell>"Переходы"</TableHeaderCell>
                                            <TableHeaderCell>"CTR"</TableHeaderCell>
                                            <TableHeaderCell>"Позиция"</TableHeaderCell>
                                            <TableHeaderCell>"Заказы из поиска"</TableHeaderCell>
                                            <TableHeaderCell>"Топ-запросы"</TableHeaderCell>
                                        </TableRow>
                                    </TableHeader>
                                    <TableBody>
                                        <For
                                            each=move || lines.clone()
                                            key=|l| l.nm_id
                                            children=move |l| {
                                                let queries = l
                                                    .top_queries
                                                    .iter()
                                                    .take(5)
                                                    .map(|q| format!("{} (частота {})", q.text, q.frequency))
                                                    .collect::<Vec<_>>()
                                                    .join(", ");
                                                view! {
                                                    <TableRow>
                                                        <TableCell><TableCellLayout>{l.nm_id}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout truncate=true>{l.title.clone()}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout truncate=true>{l.brand_name.clone()}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout attr:style="justify-content:flex-end;">{format!("{:.1}%", l.metrics.visibility)}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout attr:style="justify-content:flex-end;">{l.metrics.open_card}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout attr:style="justify-content:flex-end;">{format!("{:.1}%", l.metrics.ctr)}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout attr:style="justify-content:flex-end;">{format!("{:.1}", l.metrics.avg_position)}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout attr:style="justify-content:flex-end;">{l.metrics.orders}</TableCellLayout></TableCell>
                                                        <TableCell><TableCellLayout truncate=true>{queries}</TableCellLayout></TableCell>
                                                    </TableRow>
                                                }
                                            }
                                        />
                                    </TableBody>
                                </Table>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div class="alert">"Документ не найден."</div> }.into_any()
                    }
                }}
            </div>
        </PageFrame>
    }
}
