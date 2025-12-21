# DateRangePicker - Руководство по использованию

## Обзор

`DateRangePicker` - переиспользуемый компонент для выбора периода дат, стилизованный в соответствии с Thaw UI. Компонент объединяет функциональность выбора дат с быстрыми кнопками для популярных периодов.

## Возможности

- **Два поля ввода дат**: нативные `input[type="date"]` для выбора начальной и конечной даты
- **Быстрые кнопки**:
  - `-1M` - предыдущий месяц
  - `0M` - текущий месяц
  - `⋯` - произвольный месяц/год через модальное окно
- **Стилизация Thaw UI**: использует компоненты Button, Space, Select из Thaw
- **CSS переменные**: поддержка тем через CSS-переменные Thaw

## Местоположение

```
crates/frontend/src/shared/components/date_range_picker.rs
```

## API

### Параметры

```rust
#[component]
pub fn DateRangePicker(
    /// Значение даты "от" в формате yyyy-mm-dd
    #[prop(into)]
    date_from: Signal<String>,

    /// Значение даты "до" в формате yyyy-mm-dd
    #[prop(into)]
    date_to: Signal<String>,

    /// Callback при изменении диапазона дат (from, to)
    on_change: Callback<(String, String)>,

    /// Опциональная метка для компонента
    #[prop(optional)]
    label: Option<String>,
) -> impl IntoView
```

### Типы данных

- **date_from/date_to**: Signal<String> в формате `"yyyy-mm-dd"` (например, `"2024-12-21"`)
- **on_change**: Callback, который получает кортеж `(from: String, to: String)`
- **label**: Опциональная строка для отображения метки слева от компонента

## Примеры использования

### Базовый пример

```rust
use crate::shared::components::date_range_picker::DateRangePicker;

let (date_from, set_date_from) = signal(String::new());
let (date_to, set_date_to) = signal(String::new());

view! {
    <DateRangePicker
        date_from=date_from
        date_to=date_to
        on_change=Callback::new(move |(from, to)| {
            set_date_from.set(from);
            set_date_to.set(to);
        })
    />
}
```

### С меткой

```rust
view! {
    <DateRangePicker
        date_from=date_from
        date_to=date_to
        on_change=Callback::new(move |(from, to)| {
            // Обработка изменения дат
        })
        label="Период:".to_string()
    />
}
```

### Интеграция с состоянием страницы (из a016_ym_returns)

```rust
use crate::shared::components::date_range_picker::DateRangePicker;

// В компоненте списка
view! {
    <div class="form__group">
        <DateRangePicker
            date_from=Signal::derive(move || state.get().date_from)
            date_to=Signal::derive(move || state.get().date_to)
            on_change=Callback::new(move |(from, to)| {
                state.update(|s| {
                    s.date_from = from;
                    s.date_to = to;
                    s.page = 0; // Сброс пагинации
                });
                load_data(); // Перезагрузка данных с новыми датами
            })
            label="Период:".to_string()
        />
    </div>
}
```

## Компоненты Thaw UI

Компонент использует следующие элементы из Thaw UI:

- `Button` (с `ButtonAppearance::Subtle` и `ButtonSize::Small`)
- `Space` (с `SpaceGap::Small` для расположения кнопок)
- `Select` (для выбора месяца в модальном окне)

## Стилизация

Компонент использует CSS-переменные Thaw для поддержки тем:

- `--colorNeutralForeground1` - основной цвет текста
- `--colorNeutralForeground2` - вторичный цвет текста
- `--colorNeutralBackground1` - фон элементов
- `--colorNeutralStroke1` - цвет границ
- `--borderRadiusMedium` - радиус скругления
- `--borderRadiusLarge` - радиус для модального окна
- `--shadow64` - тень для модального окна

## Логика работы

### Кнопка "Текущий месяц" (0M)

Устанавливает период с первого до последнего дня текущего месяца:

- Начало: 1 число текущего месяца
- Конец: последний день текущего месяца

### Кнопка "Предыдущий месяц" (-1M)

Устанавливает период с первого до последнего дня предыдущего месяца:

- Если текущий месяц январь → декабрь предыдущего года
- Иначе → предыдущий месяц текущего года

### Кнопка "Произвольный период" (⋯)

Открывает модальное окно с:

- Выпадающим списком месяцев (Январь-Декабрь)
- Полем ввода года (2020-2030)
- Кнопками "Применить" и "Отмена"

При нажатии "Применить" устанавливается период с первого до последнего дня выбранного месяца/года.

## Миграция с DateInput + MonthSelector

### Старый код

```rust
<div style="display: flex; gap: var(--spacing-xs); ...">
    <DateInput
        value=Signal::derive(move || state.get().date_from)
        on_change=move |val| { /* ... */ }
    />
    <span>" — "</span>
    <DateInput
        value=Signal::derive(move || state.get().date_to)
        on_change=move |val| { /* ... */ }
    />
    <MonthSelector
        on_select=Callback::new(move |(from, to)| { /* ... */ })
    />
</div>
```

### Новый код

```rust
<DateRangePicker
    date_from=Signal::derive(move || state.get().date_from)
    date_to=Signal::derive(move || state.get().date_to)
    on_change=Callback::new(move |(from, to)| {
        state.update(|s| {
            s.date_from = from;
            s.date_to = to;
            s.page = 0;
        });
        load_data();
    })
    label="Период:".to_string()
/>
```

## Преимущества

1. **Меньше кода**: один компонент вместо трех элементов
2. **Единый стиль**: консистентная стилизация с Thaw UI
3. **Переиспользуемость**: легко использовать на разных страницах
4. **Простая миграция**: замена старых компонентов занимает минуты
5. **Лучший UX**: компактный и интуитивный интерфейс

## Где используется

### Реализовано

- ✅ `a016_ym_returns/ui/list/mod.rs` - Возвраты Яндекс Маркет

### Можно мигрировать

Компонент может быть использован на всех страницах, где нужен выбор периода дат:

- Списки продаж
- Списки транзакций
- Отчеты по периодам
- Фильтры по датам

## Зависимости

```toml
[dependencies]
leptos = "0.8"
thaw = "0.5.0-beta"
chrono = "0.4"
```

## Примечания

- Компонент работает с датами в формате ISO 8601 (`yyyy-mm-dd`)
- Браузер автоматически отображает даты в локальном формате (dd.mm.yyyy для RU)
- Модальное окно использует fixed positioning с z-index: 1000
- Компонент адаптируется к теме через CSS-переменные Thaw
