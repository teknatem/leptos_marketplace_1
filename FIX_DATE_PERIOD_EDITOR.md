# Исправление ошибки в редакторе периода (DatePeriodTab)

## Проблема

При редактировании периода в универсальном дашборде (`universal_dashboard`) возникала ошибка:

```
panicked at reactive_graph/src/traits.rs:394:39:
you tried to access a reactive value which was defined at ..., but it has already been disposed.
```

Ошибка возникала при переключении между режимами "Быстрый выбор" и "Свой период" в редакторе условий фильтрации по дате.

## Причина

В компоненте `DatePeriodTab` (файл: `crates/frontend/src/shared/universal_dashboard/ui/condition_editor/tabs/date_period.rs`) использовались замыкания для реактивных свойств кнопок внутри компонента `Show` с `For`.

При переключении между режимами:

1. `Show` уничтожал и пересоздавал контент
2. `For` создавал новые кнопки с замыканиями `move ||`
3. Старые реактивные замыкания становились disposed, но кнопки все еще пытались к ним обратиться

## Решение

Внесены следующие изменения:

### 1. Создан отдельный компонент `PresetButton`

- Изолирует реактивность каждой кнопки
- Использует `Memo` для вычисления `appearance`
- Принимает `preset_value`, `selected_preset` и `on_select` как пропсы

### 2. Заменена динамическая генерация на статическую

- Вместо `For` используется `.iter().map().collect_view()` при инициализации
- Кнопки создаются один раз и не пересоздаются при переключении режимов

### 3. Использовано CSS-скрытие вместо `Show`

- Вместо уничтожения/пересоздания компонентов используется `style:display`
- Оба раздела (preset buttons и custom inputs) всегда существуют в DOM
- При переключении меняется только видимость через `display: flex/none`

## Ключевые изменения в коде

```rust
// Отдельный компонент для кнопки пресета
#[component]
fn PresetButton(
    preset_value: DatePreset,
    #[prop(into)] selected_preset: Signal<Option<DatePreset>>,
    on_select: Callback<DatePreset>,
) -> impl IntoView {
    let appearance = Memo::new(move |_| {
        if selected_preset.get() == Some(preset_value) {
            ButtonAppearance::Primary
        } else {
            ButtonAppearance::Secondary
        }
    });
    // ...
}

// Статическое создание кнопок
let preset_buttons_view = DatePreset::all()
    .iter()
    .map(|&p| view! { <PresetButton ... /> })
    .collect_view();

// CSS-скрытие вместо Show
<div style:display=move || if use_preset.get() { "flex" } else { "none" }>
    {preset_buttons_view}
</div>
```

## Измененный файл

- `crates/frontend/src/shared/universal_dashboard/ui/condition_editor/tabs/date_period.rs`

## Результат

Ошибка больше не возникает при:

- Переключении между "Быстрый выбор" и "Свой период"
- Множественных переключениях туда-обратно
- Выборе различных предустановленных периодов
- Повторном открытии редактора условий

## Принципы решения

1. **Изоляция реактивности** - каждый компонент управляет своими реактивными зависимостями
2. **Стабильность компонентов** - не уничтожаем/пересоздаем, а скрываем/показываем
3. **Использование Memo** - для вычисляемых значений, зависящих от сигналов
