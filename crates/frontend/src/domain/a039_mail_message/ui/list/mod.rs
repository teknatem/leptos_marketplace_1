use crate::shared::api_utils::api_base;
use crate::shared::icons::icon;
use crate::shared::page_frame::PageFrame;
use contracts::domain::a039_mail_message::aggregate::MailMessage;
use leptos::prelude::*;
use thaw::*;

/// Читаемый статус письма.
fn status_ru(s: &str) -> &'static str {
    match s {
        "received" => "Получено",
        "prepared" => "Подготовлено",
        "replied" => "Отвечено",
        "rejected_unknown_sender" => "Неизв. отправитель",
        "rejected_forbidden" => "Нет прав",
        "failed" => "Ошибка",
        "overdue" => "Просрочено",
        _ => "—",
    }
}

fn direction_ru(s: &str) -> &'static str {
    match s {
        "inbound" => "Входящее",
        "outbound" => "Исходящее",
        _ => "—",
    }
}

#[component]
#[allow(non_snake_case)]
pub fn MailMessageList() -> impl IntoView {
    let (items, set_items) = signal::<Vec<MailMessage>>(Vec::new());
    let (error, set_error) = signal::<Option<String>>(None);

    let fetch = move || {
        wasm_bindgen_futures::spawn_local(async move {
            match fetch_messages().await {
                Ok(v) => {
                    set_items.set(v);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    fetch();

    view! {
        <PageFrame page_id="a039_mail_message--list" category="list">
            <Flex justify=FlexJustify::SpaceBetween align=FlexAlign::Center>
                <h1 style="font-size: 24px; font-weight: bold;">{"Письма (журнал)"}</h1>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| fetch()
                >
                    {icon("refresh")}
                    " Обновить"
                </Button>
            </Flex>
            <div style="margin-top: 16px;">
            {move || error.get().map(|e| view! {
                <div style="padding: 12px; background: var(--color-error-50); border: 1px solid var(--color-error-100); border-radius: 8px; display: flex; align-items: center; gap: 8px;">
                    <span style="color: var(--color-error); font-size: 18px;">"⚠"</span>
                    <span style="color: var(--color-error);">{e}</span>
                </div>
            })}
            </div>

            <Table>
                <TableHeader>
                    <TableRow>
                        <TableHeaderCell min_width=100.0>"Направление"</TableHeaderCell>
                        <TableHeaderCell min_width=140.0>"Статус"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=180.0>"От"</TableHeaderCell>
                        <TableHeaderCell resizable=true min_width=240.0>"Тема"</TableHeaderCell>
                        <TableHeaderCell min_width=120.0>"Интент"</TableHeaderCell>
                        <TableHeaderCell min_width=140.0>"Агент"</TableHeaderCell>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    {move || items.get().into_iter().map(|m| {
                        view! {
                            <TableRow>
                                <TableCell><TableCellLayout>{direction_ru(&m.direction)}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{status_ru(&m.status)}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{m.from_addr.clone()}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{m.base.description.clone()}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{m.intent.clone().unwrap_or_default()}</TableCellLayout></TableCell>
                                <TableCell><TableCellLayout>{m.agent_type.clone().unwrap_or_default()}</TableCellLayout></TableCell>
                            </TableRow>
                        }
                    }).collect_view()}
                </TableBody>
            </Table>
        </PageFrame>
    }
}

async fn fetch_messages() -> Result<Vec<MailMessage>, String> {
    use wasm_bindgen::JsCast;
    use web_sys::{Request, RequestInit, RequestMode, Response};

    let opts = RequestInit::new();
    opts.set_method("GET");
    opts.set_mode(RequestMode::Cors);

    let url = format!("{}/api/a039-mail-message", api_base());
    let request = Request::new_with_str_and_init(&url, &opts).map_err(|e| format!("{e:?}"))?;
    request
        .headers()
        .set("Accept", "application/json")
        .map_err(|e| format!("{e:?}"))?;

    let window = web_sys::window().ok_or_else(|| "no window".to_string())?;
    let resp_value = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("{e:?}"))?;
    let resp: Response = resp_value.dyn_into().map_err(|e| format!("{e:?}"))?;
    if !resp.ok() {
        return Err(format!("HTTP {}", resp.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(resp.text().map_err(|e| format!("{e:?}"))?)
        .await
        .map_err(|e| format!("{e:?}"))?;
    let text: String = text.as_string().ok_or_else(|| "bad text".to_string())?;
    let data: Vec<MailMessage> = serde_json::from_str(&text).map_err(|e| format!("{e}"))?;
    Ok(data)
}
