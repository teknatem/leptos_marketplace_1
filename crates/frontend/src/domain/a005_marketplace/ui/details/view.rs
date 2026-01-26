use super::view_model::MarketplaceDetailsViewModel;
use crate::shared::icons::icon;
use contracts::enums::marketplace_type::MarketplaceType;
use leptos::prelude::*;
use std::rc::Rc;

#[component]
pub fn MarketplaceDetails(
    id: Option<String>,
    #[prop(optional)]
    readonly: bool,
    on_saved: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let vm = MarketplaceDetailsViewModel::new();
    vm.load_if_needed(id);

    // Clone vm for multiple closures
    let vm_clone = vm.clone();
    let vm_for_header = vm_clone.clone();
    let vm_for_error = vm_clone.clone();
    let vm_for_actions = vm_clone.clone();

    let form_view = {
        let vm_clone = vm_clone.clone();
        move || {
            view! {
                <div class="detail-form">
                    <div class="detail-form-content">
                        <div class="form__group">
                            <label class="form__label" for="description">{"Наименование"}</label>
                            <input
                                class="form__input"
                                type="text"
                                id="description"
                                prop:readonly=readonly
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().description
                                }
                                on:input={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        vm.form.update(|f| f.description = event_target_value(&ev));
                                    }
                                }
                                placeholder="Введите наименование маркетплейса"
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label" for="url">{"URL"}</label>
                            <input
                                class="form__input"
                                type="text"
                                id="url"
                                prop:readonly=readonly
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().url
                                }
                                on:input={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        vm.form.update(|f| f.url = event_target_value(&ev));
                                    }
                                }
                                placeholder="https://example.com"
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label" for="marketplace_type">{"Тип маркетплейса"}</label>
                            <select
                                class="form__select"
                                id="marketplace_type"
                                disabled=readonly
                                on:change={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        let value = event_target_value(&ev);
                                        let mp_type = if value.is_empty() {
                                            None
                                        } else {
                                            MarketplaceType::from_code(&value)
                                        };
                                        vm.form.update(|f| {
                                            f.marketplace_type = mp_type;
                                            // Sync code with type
                                            if let Some(t) = mp_type {
                                                f.code = Some(t.code().to_string());
                                            }
                                        });
                                    }
                                }
                            >
                                <option value="" selected={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().marketplace_type.is_none()
                                }>
                                    {"-- Не выбрано --"}
                                </option>
                                {MarketplaceType::all().into_iter().map(|mp_type| {
                                    let code = mp_type.code();
                                    let name = mp_type.display_name();
                                    let vm_for_selected = vm_clone.clone();
                                    view! {
                                        <option
                                            value={code}
                                            selected={
                                                move || {
                                                    vm_for_selected.form.get().marketplace_type
                                                        .map(|t| t == mp_type)
                                                        .unwrap_or(false)
                                                }
                                            }
                                        >
                                            {name}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                        </div>

                        <div class="form__group">
                            <label class="form__label" for="logo_path">{"Путь к логотипу"}</label>
                            <input
                                class="form__input"
                                type="text"
                                id="logo_path"
                                prop:readonly=readonly
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().logo_path.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        let value = event_target_value(&ev);
                                        vm.form.update(|f| {
                                            f.logo_path = if value.is_empty() { None } else { Some(value) };
                                        });
                                    }
                                }
                                placeholder="/assets/images/logo.svg"
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label" for="acquiring_fee_pro">{"Эквайринг, %"}</label>
                            <input
                                class="form__input"
                                type="number"
                                step="0.01"
                                min="0"
                                max="100"
                                id="acquiring_fee_pro"
                                prop:readonly=readonly
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || format!("{:.2}", vm.form.get().acquiring_fee_pro)
                                }
                                on:input={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        let value: f64 = event_target_value(&ev).parse().unwrap_or(0.0);
                                        vm.form.update(|f| f.acquiring_fee_pro = value);
                                    }
                                }
                                placeholder="0.00"
                            />
                        </div>

                        <div class="form__group">
                            <label class="form__label" for="comment">{"Комментарий"}</label>
                            <textarea
                                class="form__textarea"
                                id="comment"
                                prop:readonly=readonly
                                prop:value={
                                    let vm = vm_clone.clone();
                                    move || vm.form.get().comment.clone().unwrap_or_default()
                                }
                                on:input={
                                    let vm = vm_clone.clone();
                                    move |ev| {
                                        if readonly {
                                            return;
                                        }
                                        let value = event_target_value(&ev);
                                        vm.form.update(|f| {
                                            f.comment = if value.is_empty() { None } else { Some(value) };
                                        });
                                    }
                                }
                                placeholder="Введите дополнительную информацию (необязательно)"
                                rows="3"
                            />
                        </div>
                    </div>
                </div>
            }
            .into_any()
        }
    };

    view! {
        <div class="details-container marketplace-details">
            {move || {
                if readonly {
                    view! {
                        <div class="modal-header">
                            <h3 class="modal-title">"Маркетплейс"</h3>
                            <div class="modal-header-actions">
                                <button class="button button--secondary" on:click=move |_| on_cancel.run(())>
                                    {icon("x")} " Закрыть"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="modal-header">
                            <h3 class="modal-title">
                                {
                                    let vm = vm_for_header.clone();
                                    move || if vm.is_edit_mode()() { "Редактирование маркетплейса" } else { "Новый маркетплейс" }
                                }
                            </h3>
                            <div class="modal-header-actions">
                                <button
                                    class="button button--primary"
                                    on:click={
                                        let vm = vm_for_actions.clone();
                                        let on_saved = on_saved.clone();
                                        move |_| {
                                            let on_saved_cb = on_saved.clone();
                                            let on_saved_rc: Rc<dyn Fn(())> =
                                                Rc::new(move |_| on_saved_cb.run(()));
                                            vm.save_command(on_saved_rc);
                                        }
                                    }
                                    disabled={
                                        let vm = vm_for_actions.clone();
                                        move || !vm.is_form_valid()()
                                    }
                                >
                                    {icon("save")}
                                    {
                                        let vm = vm_clone.clone();
                                        move || if vm.is_edit_mode()() { "Сохранить" } else { "Создать" }
                                    }
                                </button>
                                <button class="button button--secondary" on:click=move |_| on_cancel.run(())>
                                    {icon("x")} " Закрыть"
                                </button>
                            </div>
                        </div>
                    }.into_any()
                }
            }}

            {
                let vm = vm_for_error.clone();
                move || vm.error.get().map(|e| view! {
                    <div class="warning-box warning-box--error">
                        <span class="warning-box__icon">"⚠"</span>
                        <span class="warning-box__text">{e}</span>
                    </div>
                })
            }

            {move || {
                if readonly {
                    view! {
                        <div class="modal-body">
                            {form_view()}
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="modal-body">
                            {form_view()}
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
