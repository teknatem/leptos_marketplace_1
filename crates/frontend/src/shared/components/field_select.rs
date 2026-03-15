use leptos::prelude::*;

#[component]
pub fn FieldSelect(
    #[prop(into)]
    value: Signal<String>,
    #[prop(into)]
    options: Signal<Vec<(String, String)>>,
    on_change: Callback<String>,
    #[prop(optional, into)]
    id: MaybeProp<String>,
    #[prop(optional, into)]
    placeholder: MaybeProp<String>,
    #[prop(optional, into)]
    class: MaybeProp<String>,
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    let open = RwSignal::new(false);

    let trigger_id = move || id.get().unwrap_or_default();
    let trigger_class = move || match class.get() {
        Some(extra) if !extra.is_empty() => format!("field-select {}", extra),
        _ => "field-select".to_string(),
    };
    let placeholder_text =
        move || placeholder.get().unwrap_or_else(|| "Выберите значение".to_string());
    let selected_label = Signal::derive(move || {
        let current_value = value.get();
        options
            .get()
            .into_iter()
            .find(|(option_value, _)| *option_value == current_value)
            .map(|(_, label)| label)
            .filter(|label| !label.is_empty())
            .unwrap_or_else(placeholder_text)
    });

    view! {
        <div class=trigger_class>
            <button
                type="button"
                id=trigger_id
                class="field-select__trigger"
                class:field-select__trigger--placeholder=move || value.get().is_empty()
                aria-haspopup="listbox"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                disabled=disabled
                on:click=move |_| {
                    if !disabled {
                        open.update(|current| *current = !*current);
                    }
                }
            >
                <span class="field-select__value">{move || selected_label.get()}</span>
                <span class="field-select__chevron" aria-hidden="true">
                    <svg width="14" height="14" viewBox="0 0 20 20" fill="none">
                        <path
                            d="M5 7.5L10 12.5L15 7.5"
                            stroke="currentColor"
                            stroke-width="1.7"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                        />
                    </svg>
                </span>
            </button>

            <Show when=move || open.get()>
                <button
                    type="button"
                    class="field-select__backdrop"
                    aria-label="Закрыть список"
                    on:click=move |_| open.set(false)
                />

                <div class="field-select__popover" role="listbox">
                    <For
                        each=move || options.get()
                        key=|(option_value, _)| option_value.clone()
                        children=move |(option_value, option_label)| {
                            let is_selected = Signal::derive({
                                let option_value = option_value.clone();
                                move || value.get() == option_value
                            });
                            let option_value_for_click = option_value.clone();

                            view! {
                                <button
                                    type="button"
                                    class="field-select__option"
                                    class:field-select__option--selected=move || is_selected.get()
                                    on:click=move |_| {
                                        on_change.run(option_value_for_click.clone());
                                        open.set(false);
                                    }
                                >
                                    <span class="field-select__option-label">{option_label}</span>
                                    <Show when=move || is_selected.get()>
                                        <span class="field-select__option-check" aria-hidden="true">
                                            <svg width="14" height="14" viewBox="0 0 20 20" fill="none">
                                                <path
                                                    d="M4.5 10.5L8 14L15.5 6.5"
                                                    stroke="currentColor"
                                                    stroke-width="1.8"
                                                    stroke-linecap="round"
                                                    stroke-linejoin="round"
                                                />
                                            </svg>
                                        </span>
                                    </Show>
                                </button>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}
