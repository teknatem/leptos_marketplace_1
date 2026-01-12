use leptos::prelude::*;
use thaw::*;

#[component]
pub fn DimensionInput(
    #[prop(into)] id: String,
    #[prop(into)] label: String,
    #[prop(into)] placeholder: String,
    maxlength: u32,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    options: Signal<Vec<String>>,
) -> impl IntoView {
    // Local editable state (Thaw Input needs RwSignal)
    let input_value = RwSignal::new(String::new());
    let is_open = RwSignal::new(false);

    // Sync external value -> input
    Effect::new(move || {
        let v = value.get();
        input_value.set(v);
    });

    // Sync Input -> callback (avoid initial echo)
    let last_sent = StoredValue::new(String::new());
    let first_run = StoredValue::new(true);
    Effect::new(move || {
        let v = input_value.get();
        if first_run.get_value() {
            first_run.set_value(false);
            last_sent.set_value(v);
            return;
        }
        if last_sent.get_value() == v {
            return;
        }
        last_sent.set_value(v.clone());
        on_change.run(v);
    });

    let on_clear = move |_| {
        input_value.set(String::new());
        on_change.run(String::new());
        is_open.set(false);
    };

    let toggle_dropdown = move |_| {
        is_open.update(|v| *v = !*v);
    };

    let select_option = move |opt: String| {
        input_value.set(opt.clone());
        on_change.run(opt);
        is_open.set(false);
    };

    view! {
        <div class="form__group">
            <label class="form__label" for={id.clone()}>{label}</label>

            <div style="position: relative;">
                <Input
                    value=input_value
                    placeholder=placeholder
                    attr:id=id.clone()
                    attr:maxlength=maxlength.to_string()
                    attr:style="width: 100%; padding-right: 0px;"
                >
                    <InputSuffix slot>
                        <div style="display: flex; gap: 0px;">
                            <Button
                                appearance=ButtonAppearance::Subtle
                                shape=ButtonShape::Square
                                size=ButtonSize::Small
                                on_click=on_clear
                                attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                attr:title="Очистить"
                            >
                                "✕"
                            </Button>
                            <Button
                                appearance=ButtonAppearance::Subtle
                                shape=ButtonShape::Square
                                size=ButtonSize::Small
                                on_click=toggle_dropdown
                                attr:style="width: 28px; height: 28px; min-width: 28px; padding: 0; display: flex; align-items: center; justify-content: center;"
                                attr:title="Выбрать из списка"
                            >
                                "▼"
                            </Button>
                        </div>
                    </InputSuffix>
                </Input>

                {move || {
                    if !is_open.get() {
                        return view! { <></> }.into_any();
                    }

                    let current = input_value.get().to_lowercase();
                    let opts = options
                        .get()
                        .into_iter()
                        .filter(|o| {
                            if current.trim().is_empty() {
                                true
                            } else {
                                o.to_lowercase().contains(&current)
                            }
                        })
                        .take(50)
                        .collect::<Vec<_>>();

                    view! {
                        <div
                            style="
                                position: absolute;
                                top: calc(100% + 4px);
                                left: 0;
                                right: 0;
                                max-height: 220px;
                                overflow-y: auto;
                                background: var(--color-surface);
                                border: 1px solid var(--color-border);
                                border-radius: var(--radius-md);
                                box-shadow: var(--shadow-md);
                                z-index: 1000;
                            "
                        >
                            {if opts.is_empty() {
                                view! { <div style="padding: 8px 12px; color: var(--color-text-tertiary);">"Нет совпадений"</div> }.into_any()
                            } else {
                                opts.into_iter().map(|opt| {
                                    let opt2 = opt.clone();
                                    view! {
                                        <div
                                            style="padding: 8px 12px; cursor: pointer; border-bottom: 1px solid var(--color-border-light);"
                                            on:mousedown=move |_| select_option(opt2.clone())
                                        >
                                            {opt}
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}
