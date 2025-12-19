use super::traits::TableDisplayable;
use leptos::html::Tr;
use leptos::prelude::*;

/// Универсальный компонент для выбора агрегата из списка
///
/// Поддерживает:
/// - Отображение списка элементов
/// - Предвыбор элемента через initial_selected_id
/// - Автоскролл к выбранному элементу
/// - Клик для выбора, двойной клик для подтверждения
#[component]
pub fn GenericAggregatePicker<T>(
    /// Список элементов для выбора
    items: ReadSignal<Vec<T>>,
    /// Ошибка загрузки (если есть)
    #[prop(optional)]
    error: Option<ReadSignal<Option<String>>>,
    /// Индикатор загрузки
    #[prop(optional)]
    loading: Option<ReadSignal<bool>>,
    /// ID элемента, который должен быть выбран при открытии
    initial_selected_id: Option<String>,
    /// Callback при подтверждении выбора
    on_confirm: impl Fn(Option<T>) + 'static + Clone + Send,
    /// Callback при отмене
    on_cancel: impl Fn(()) + 'static + Clone + Send,
    /// Заголовок модального окна
    #[prop(optional)]
    title: Option<String>,
) -> impl IntoView
where
    T: TableDisplayable + Clone + Send + Sync + 'static,
{
    let (selected_id, set_selected_id) = signal::<Option<String>>(initial_selected_id.clone());
    let _title = title.unwrap_or_else(|| "Выбор элемента".to_string());

    let loading_signal = loading.unwrap_or_else(|| {
        let (r, _) = signal(false);
        r
    });
    let error_signal = error.unwrap_or_else(|| {
        let (r, _) = signal(None);
        r
    });

    // Реф для автоскролла к выбранному элементу
    let selected_row_ref = NodeRef::<Tr>::new();

    // Автоскролл к выбранному элементу после рендеринга
    Effect::new(move |_| {
        if selected_id.get().is_some() && !loading_signal.get() {
            if let Some(element) = selected_row_ref.get() {
                let _ = element.scroll_into_view_with_bool(true);
            }
        }
    });

    let handle_confirm = {
        let on_confirm = on_confirm.clone();
        move |_| {
            let selected = selected_id.get();
            if let Some(id) = selected {
                items.with(|items_vec| {
                    if let Some(item) = items_vec.iter().find(|i| i.id() == id) {
                        on_confirm(Some(item.clone()));
                    } else {
                        on_confirm(None);
                    }
                });
            } else {
                on_confirm(None);
            }
        }
    };

    let handle_row_click = move |item_id: String| {
        set_selected_id.set(Some(item_id));
    };

    let on_confirm_dblclick = on_confirm.clone();

    view! {
        <div class="picker">
            <div class="picker__content">
                {move || {
                    if loading_signal.get() {
                        view! { <div class="picker__loading">"Загрузка..."</div> }.into_any()
                    } else if let Some(err) = error_signal.get() {
                        view! {
                            <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100);">
                                <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                                <span class="warning-box__text" style="color: var(--color-error);">
                                    {"Ошибка загрузки: "} {err}
                                </span>
                            </div>
                        }.into_any()
                    } else {
                        items.with(|items_vec| {
                            if items_vec.is_empty() {
                                view! {
                                    <div class="picker__empty">"Нет доступных элементов"</div>
                                }.into_any()
                            } else {
                                view! {
                                    <table class="picker__table table__data table--striped">
                                        <thead class="table__head">
                                            <tr>
                                                <th class="table__header-cell">"Описание"</th>
                                                <th class="table__header-cell">"Код"</th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            {items_vec.iter().enumerate().map(|(_idx, item)| {
                                                let item_id = item.id();
                                                let item_id_for_selected = item_id.clone();
                                                let item_id_for_selected2 = item_id.clone();
                                                let item_id_for_click = item_id.clone();
                                                let item_for_dblclick = item.clone();
                                                let on_confirm_clone = on_confirm_dblclick.clone();
                                                let is_initially_selected = initial_selected_id.as_ref() == Some(&item_id);

                                                view! {
                                                    <tr
                                                        node_ref=if is_initially_selected { selected_row_ref } else { NodeRef::new() }
                                                        class="picker__row table__row"
                                                        class:picker__row--selected=move || selected_id.get().as_ref() == Some(&item_id_for_selected)
                                                        class:table__row--selected=move || selected_id.get().as_ref() == Some(&item_id_for_selected2)
                                                        on:click=move |_| handle_row_click(item_id_for_click.clone())
                                                        on:dblclick=move |_| on_confirm_clone(Some(item_for_dblclick.clone()))
                                                    >
                                                        <td class="table__cell">{item.description()}</td>
                                                        <td class="table__cell">{item.code()}</td>
                                                    </tr>
                                                }
                                            }).collect_view()}
                                        </tbody>
                                    </table>
                                }.into_any()
                            }
                        })
                    }
                }}
            </div>

            <div class="picker__actions">
                <button
                    class="button button--primary"
                    on:click=handle_confirm
                    disabled=move || selected_id.get().is_none()
                >
                    "Выбрать"
                </button>
                <button
                    class="button button--secondary"
                    on:click=move |_| on_cancel(())
                >
                    "Отмена"
                </button>
            </div>
        </div>
    }
}
