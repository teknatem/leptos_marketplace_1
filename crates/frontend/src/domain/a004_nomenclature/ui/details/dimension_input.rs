use leptos::prelude::*;
use leptos::ev::MouseEvent;
use wasm_bindgen::JsCast;

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
    let (is_open, set_is_open) = signal(false);
    let (filter_text, set_filter_text) = signal(String::new());
    
    // Filtered options based on current input
    let filtered_options = move || {
        let filter = filter_text.get().to_lowercase();
        if filter.is_empty() {
            options.get()
        } else {
            options.get()
                .into_iter()
                .filter(|opt| opt.to_lowercase().contains(&filter))
                .collect()
        }
    };

    let on_input = move |ev: leptos::ev::Event| {
        let val = event_target_value(&ev);
        set_filter_text.set(val.clone());
        on_change.run(val);
    };

    let on_clear = move |_: MouseEvent| {
        set_filter_text.set(String::new());
        on_change.run(String::new());
        set_is_open.set(false);
    };

    let toggle_dropdown = move |_: MouseEvent| {
        set_is_open.update(|open| *open = !*open);
    };

    let select_option = move |option: String| {
        set_filter_text.set(option.clone());
        on_change.run(option);
        set_is_open.set(false);
    };

    let on_focus = move |_| {
        set_is_open.set(true);
    };

    // Sync value with filter_text
    Effect::new(move || {
        let v = value.get();
        set_filter_text.set(v);
    });

    // Close dropdown when clicking outside (with small delay for mousedown to fire first)
    let on_blur = move |_| {
        leptos::task::spawn_local(async move {
            gloo_timers::future::TimeoutFuture::new(150).await;
            set_is_open.set(false);
        });
    };

    view! {
        <div class="form-group">
            <label for={id.clone()}>{label}</label>
            <div style="position: relative;">
                <input
                    type="text"
                    id={id.clone()}
                    maxlength={maxlength.to_string()}
                    prop:value={move || filter_text.get()}
                    on:input=on_input
                    on:focus=on_focus
                    on:blur=on_blur
                    placeholder={placeholder}
                    style="padding-right: 70px;"
                />
                <div style="position: absolute; right: 8px; top: 50%; transform: translateY(-50%); display: flex; gap: 4px;">
                    // Clear button
                    <button
                        type="button"
                        on:mousedown=on_clear
                        style="
                            border: none;
                            background: transparent;
                            cursor: pointer;
                            padding: 4px 8px;
                            color: #666;
                            font-size: 16px;
                            display: flex;
                            align-items: center;
                        "
                        title="Очистить"
                    >
                        "✕"
                    </button>
                    // Dropdown button
                    <button
                        type="button"
                        on:mousedown=toggle_dropdown
                        style="
                            border: none;
                            background: transparent;
                            cursor: pointer;
                            padding: 4px 8px;
                            color: #666;
                            font-size: 14px;
                            display: flex;
                            align-items: center;
                        "
                        title="Выбрать из списка"
                    >
                        "▼"
                    </button>
                </div>
                {move || if is_open.get() {
                    let opts = filtered_options();
                    view! {
                        <div style="
                            position: absolute;
                            top: 100%;
                            left: 0;
                            right: 0;
                            max-height: 200px;
                            overflow-y: auto;
                            background: white;
                            border: 1px solid #ccc;
                            border-radius: 4px;
                            box-shadow: 0 2px 8px rgba(0,0,0,0.15);
                            z-index: 1000;
                            margin-top: 2px;
                        ">
                            {if opts.is_empty() {
                                view! {
                                    <div style="padding: 8px 12px; color: #999; font-style: italic;">
                                        "Нет совпадений"
                                    </div>
                                }.into_any()
                            } else {
                                opts.into_iter().map(|option| {
                                    let opt_clone = option.clone();
                                    view! {
                                        <div
                                            on:mousedown=move |_| select_option(opt_clone.clone())
                                            style="
                                                padding: 8px 12px;
                                                cursor: pointer;
                                                border-bottom: 1px solid #eee;
                                            "
                                            on:mouseenter=move |ev| {
                                                if let Some(target) = ev.target() {
                                                    if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                                                        let _ = element.style().set_property("background-color", "#f0f0f0");
                                                    }
                                                }
                                            }
                                            on:mouseleave=move |ev| {
                                                if let Some(target) = ev.target() {
                                                    if let Ok(element) = target.dyn_into::<web_sys::HtmlElement>() {
                                                        let _ = element.style().set_property("background-color", "white");
                                                    }
                                                }
                                            }
                                        >
                                            {option}
                                        </div>
                                    }
                                }).collect_view().into_any()
                            }}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }}
            </div>
        </div>
    }
}

