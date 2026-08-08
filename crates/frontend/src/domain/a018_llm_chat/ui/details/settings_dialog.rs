//! Диалог «Настройки чата»: что показывать в ленте.
//!
//! Настройки применяются живьём (лента реагирует на сигнал) и сразу пишутся
//! в `localStorage` — отдельной кнопки «Сохранить» нет намеренно.

use super::prefs::ChatUiPrefs;
use leptos::prelude::*;
use thaw::*;

#[component]
#[allow(non_snake_case)]
pub fn ChatSettingsDialog(open: RwSignal<bool>, prefs: RwSignal<ChatUiPrefs>) -> impl IntoView {
    // Локальные сигналы под thaw `Checkbox` (он принимает Model<bool>);
    // обратная запись в `prefs` + сохранение — одним эффектом ниже.
    let initial = prefs.get_untracked();
    let show_meta_line = RwSignal::new(initial.show_meta_line);
    let show_tokens = RwSignal::new(initial.show_tokens);
    let show_tool_calls = RwSignal::new(initial.show_tool_calls);
    let show_skill_warnings = RwSignal::new(initial.show_skill_warnings);
    let show_context_events = RwSignal::new(initial.show_context_events);

    Effect::new(move |_| {
        let next = ChatUiPrefs {
            version: prefs.get_untracked().version,
            show_meta_line: show_meta_line.get(),
            show_tokens: show_tokens.get(),
            show_tool_calls: show_tool_calls.get(),
            show_skill_warnings: show_skill_warnings.get(),
            show_context_events: show_context_events.get(),
        };
        prefs.set(next);
        next.save();
    });

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Настройки чата"</DialogTitle>
                    <DialogContent>
                        <div style="display: flex; flex-direction: column; gap: 10px;">
                            <span style="font-size: 13px; opacity: 0.7;">
                                "Технические подробности ответа скрыты по умолчанию. Включите то, \
                                 что нужно видеть — настройка общая для всех чатов и запоминается \
                                 в этом браузере."
                            </span>
                            <Checkbox
                                checked=show_meta_line
                                label="Показывать мету ответа (модель, время, уверенность)"
                            />
                            <div style="padding-left: 24px;">
                                <Checkbox
                                    checked=show_tokens
                                    label="…и расход токенов"
                                />
                                <div style="font-size: 11px; opacity: 0.55; margin-top: 2px;">
                                    "Видно только когда включена мета ответа."
                                </div>
                            </div>
                            <Checkbox
                                checked=show_tool_calls
                                label="Показывать вызовы инструментов"
                            />
                            <Checkbox
                                checked=show_skill_warnings
                                label="Предупреждать о расширенных навыках"
                            />
                            <Checkbox
                                checked=show_context_events
                                label="Показывать прикрепление документов в ленте"
                            />
                        </div>
                    </DialogContent>
                    <DialogActions>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=move |_| open.set(false)
                        >
                            "Закрыть"
                        </Button>
                    </DialogActions>
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}
