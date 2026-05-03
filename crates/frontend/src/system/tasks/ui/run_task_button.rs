//! Кнопка ручного запуска регламентного задания по коду (`sys_tasks.code`), как из списка заданий.
use crate::shared::icons::icon;
use crate::system::tasks::api::{self, RunTaskNowOutcome};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use thaw::*;

#[component]
pub fn RunTaskButton(
    /// Значение поля `code` в `sys_tasks` (например `task001-wb-orders-fbs`)
    task_code: String,
    /// Подпись на кнопке
    #[prop(default = String::from("Выполнить"))]
    label: String,
) -> impl IntoView {
    let (busy, set_busy) = signal(false);
    let (hint, set_hint) = signal::<Option<String>>(None);
    let task_code = RwSignal::new(task_code);
    let label = RwSignal::new(label);

    let on_click = move |_| {
        let tc = task_code.get_untracked();
        spawn_local(async move {
            set_busy.set(true);
            set_hint.set(None);
            let tasks = match api::fetch_scheduled_tasks().await {
                Ok(t) => t,
                Err(e) => {
                    set_hint.set(Some(format!("Не удалось загрузить задания: {}", e)));
                    set_busy.set(false);
                    return;
                }
            };
            let Some(task) = tasks.into_iter().find(|t| t.code == tc) else {
                set_hint.set(Some(format!(
                    "Задание с кодом «{}» не найдено в регламентных задачах",
                    tc
                )));
                set_busy.set(false);
                return;
            };
            match api::run_task_now(&task.id).await {
                Ok(RunTaskNowOutcome::Started(_)) => {
                    set_hint.set(Some("Запущено".to_string()));
                }
                Ok(RunTaskNowOutcome::AlreadyRunning(r)) => {
                    set_hint.set(Some(format!(
                        "Уже выполняется с {}",
                        r.started_at.format("%d.%m.%Y %H:%M:%S")
                    )));
                }
                Err(e) => {
                    log!("RunTaskButton {}: {}", tc, e);
                    set_hint.set(Some(e));
                }
            }
            set_busy.set(false);
        });
    };

    view! {
        <Flex gap=FlexGap::Small align=FlexAlign::Center>
            <Button
                appearance=ButtonAppearance::Secondary
                on_click=on_click
                disabled=busy
            >
                {move || if busy.get() {
                    view! { <Spinner size=SpinnerSize::Tiny /> }.into_any()
                } else {
                    view! {
                        <Flex gap=FlexGap::Small align=FlexAlign::Center>
                            {icon("play")}
                            <span>{move || label.get()}</span>
                        </Flex>
                    }.into_any()
                }}
            </Button>
            {move || hint.get().map(|h| view! {
                <span style="font-size:12px;color:var(--color-text-secondary);max-width:280px;">{h}</span>
            })}
        </Flex>
    }
}
