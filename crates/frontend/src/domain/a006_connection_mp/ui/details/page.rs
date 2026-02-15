//! Main page component for Connection MP details

use super::tabs::GeneralTab;
use super::view_model::ConnectionMPDetailsVm;
use crate::layout::global_context::AppGlobalContext;
use crate::shared::icons::icon;
use leptos::prelude::*;
use std::rc::Rc;
use thaw::*;

#[component]
pub fn ConnectionMPDetail(
    id: Option<String>,
    #[prop(into)] on_close: Callback<()>,
) -> impl IntoView {
    let vm = ConnectionMPDetailsVm::new(id.clone());
    let tabs_store = use_context::<AppGlobalContext>().expect("AppGlobalContext not found");

    // Обновить заголовок таба после загрузки данных
    if let Some(id_val) = id.clone() {
        let stored_id = StoredValue::new(id_val);
        Effect::new({
            let vm = vm.clone();
            move || {
                let form = vm.form.get();
                if !form.description.is_empty() {
                    let tab_key = format!("a006_connection_mp_detail_{}", stored_id.get_value());
                    let tab_title = format!("Подключение: {}", form.description);
                    tabs_store.update_tab_title(&tab_key, &tab_title);
                }
            }
        });
    }

    let vm_header = vm.clone();
    let vm_content = vm.clone();

    view! {
        <div class="page page--detail">
            <Header vm=vm_header id=id on_close=on_close />

            <div class="page__content">
                {move || {
                    if let Some(err) = vm.error.get() {
                        view! {
                            <div class="warning-box" style="background: var(--color-error-50); border-color: var(--color-error-100); margin: var(--spacing-md);">
                                <span class="warning-box__icon" style="color: var(--color-error);">"⚠"</span>
                                <span class="warning-box__text" style="color: var(--color-error);">{err}</span>
                            </div>
                        }
                        .into_any()
                    } else {
                        view! {
                            <div style="padding: var(--spacing-md);">
                                <GeneralTab vm=vm_content.clone() />
                            </div>
                        }
                        .into_any()
                    }
                }}
            </div>
        </div>
    }
}

#[component]
fn Header(
    vm: ConnectionMPDetailsVm,
    id: Option<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    let is_edit = id.is_some();
    let title = if is_edit {
        "Редактирование подключения"
    } else {
        "Новое подключение"
    };

    let vm_save = vm.clone();
    let vm_test = vm.clone();

    let handle_save = move |_| {
        let on_saved = Rc::new(move |_| {
            on_close.run(());
        });
        vm_save.save_command(on_saved);
    };

    let handle_test = move |_| {
        vm_test.test_command();
    };

    view! {
        <div class="page__header">
            <div class="page__header-left">
                <h2>{title}</h2>
            </div>
            <div class="page__header-right">
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=handle_test
                    disabled=Signal::derive(move || vm.is_testing.get())
                >
                    {icon("test")}
                    {move || if vm.is_testing.get() { " Тест..." } else { " Тест" }}
                </Button>
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=handle_save
                    disabled=Signal::derive({
                        let vm = vm.clone();
                        move || !vm.is_form_valid()()
                    })
                >
                    {icon("save")}
                    " Сохранить"
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_close.run(())
                >
                    {icon("x")}
                    " Закрыть"
                </Button>
            </div>
        </div>
    }
}
