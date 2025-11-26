# План: Универсальный пикер агрегатов v2 (просто и надёжно)

Цель: заменить специализированный `MarketplacePicker` на универсальный, минимально сложный и повторно используемый компонент выбора, с поддержкой передачи текущего значения поля для предвыбора.

Дополнение: поддерживаем два механизма пикеров — специализированный (агрегат сам рисует сложный UI) и универсальный (общая форма по полям id/code/description). На месте вызова выбираем специализированный, если он доступен, иначе — универсальный.

## Ключевые принципы

- Простота API: входом служит `Resource` с данными, без сложных generic-async в пропсах.
- Предвыбор: обязательный проп `initial_selected_id: Option<String>` устанавливает текущий выбор при открытии.
- Единый модальный сервис: показ/скрытие осуществляется централизованно.
- Минимум зависимостей: трейты и UI остаются в `frontend/src/shared`, реализации — в срезах агрегатов.
- Без `Send`-ограничений в обработчиках (WASM/Leptos).

---

## Этап 0. Два механизма пикеров и выбор

1. Определение

- Специализированный пикер: отдельный компонент внутри агрегата-источника данных с любым кастомным UI/логикой загрузки.
- Универсальный пикер: общий компонент `GenericAggregatePicker<T>` с колонками «Описание/Код» и предвыбором.

2. Простой механизм выбора на месте вызова

- Если модуль агрегата экспортирует функцию-фабрику специализированного пикера — используем её.
- Иначе используем универсальный пикер с `Resource` и `initial_selected_id`.

3. Единая точка входа (хелперы)

- В `shared/aggregate_picker.rs` добавить хелпер:
  - `open_universal_picker::<T>(modal: &ModalService, items, initial_selected_id, on_confirm, on_cancel)`.
- В агрегатах со спец. UI экспортировать:
  - `pub fn open_specialized_picker(modal: &ModalService, initial_selected_id: Option<String>, on_confirm: impl Fn(Option<Item>) + 'static + Clone, on_cancel: impl Fn(()) + 'static + Clone)`.

Это избавляет от глобальных реестров и сложных trait-обнаружений; выбор делается локально по наличию функции.

---

## Этап 1. Модальный сервис

1. Создать `crates/frontend/src/layout/modal_service.rs`:

- `ModalService` с внутренним сигналом `Signal<Option<View>>` и методами `show(View)` / `hide()`.
- `#[component] ModalProvider` — кладёт сервис в контекст.
- `#[component] ModalRenderer` — читает сигнал, рендерит оверлей/контент (через `<Portal/>`), обрабатывает Esc.

2. Подключение провайдера

- Обернуть корневой лэйаут в `<ModalProvider>` и один раз отрисовать `<ModalRenderer/>` (в `app.rs` или `layout/global_context.rs`).

---

## Этап 2. Данные и маппинг (shared/data)

1. Вынести HTTP-запросы из UI в `crates/frontend/src/shared/data/marketplace.rs`:

- `async fn fetch_marketplaces() -> Result<Vec<Marketplace>, String>`.

2. На экране/в месте использования создать `Resource<(), Result<Vec<MarketplacePickerItem>, String>>`:

- Маппинг `Marketplace -> MarketplacePickerItem` рядом с экраном (или отдельной функцией-хелпером), чтобы не смешивать домен и UI.

---

## Этап 3. Универсальный пикер (shared/aggregate_picker.rs)

1. Создать `crates/frontend/src/shared/aggregate_picker.rs` и определить минимальные трейты:

```rust
pub trait AggregatePickerResult {
    fn id(&self) -> String;
    fn display_name(&self) -> String;
}

pub trait TableDisplayable: AggregatePickerResult {
    fn code(&self) -> String;
    fn description(&self) -> String;
}
```

2. Компонент `GenericAggregatePicker<T>`:

Пропсы (минимум и по делу):

```rust
#[component]
pub fn GenericAggregatePicker<T>(
    items: Resource<(), Result<Vec<T>, String>>,
    initial_selected_id: Option<String>,
    on_confirm: impl Fn(Option<T>) + 'static + Clone,
    on_cancel: impl Fn(()) + 'static + Clone,
    render_row: Option<Rc<dyn Fn(&T, bool) -> View>>,
) -> impl IntoView
where
    T: TableDisplayable + Clone + 'static
{ /* реализация */ }
```

Поведение:

- Локальный `selected_id` и установка из `initial_selected_id` при монтировании.
- После загрузки `items` — валидация предвыбора; при отсутствии совпадения `selected_id` остаётся `None`.
- Двойной клик и Enter — подтверждают выбор; Esc — отменяет; стрелки — перемещают выделение.
- Состояния: загрузка, ошибка (с «Повторить»), пустой список.
- Разметка по умолчанию: две колонки «Описание» и «Код», плюс поддержка `render_row` для кастомной строки (например, логотип).

---

## Этап 3b. Специализированные пикеры (per-aggregate)

1. В агрегатах, где требуется кастомный UI, объявить компонент пикера и экспортировать хелпер открытия:

```rust
// пример: a005_marketplace/ui/picker/mod.rs
pub type OnConfirm<T> = Rc<dyn Fn(Option<T>)>;
pub type OnCancel = Rc<dyn Fn(())>;

pub fn open_specialized_picker(
    modal: &ModalService,
    initial_selected_id: Option<String>,
    on_confirm: OnConfirm<MarketplacePickerItem>,
    on_cancel: OnCancel,
) {
    modal.show(view! {
        <MarketplacePicker
            initial_selected_id=initial_selected_id
            on_selected=move |x| on_confirm(x)
            on_cancel=move |_| on_cancel(())
        />
    }.into_view());
}
```

2. Требования совпадают с универсальным пикером: поддержка `initial_selected_id`, Enter/Esc/двойной клик, индикаторы загрузки/ошибок.

---

## Этап 4. Интеграция с агрегатом Marketplace

1. Реализовать трейты для `MarketplacePickerItem` в `crates/frontend/src/domain/a005_marketplace/ui/picker/mod.rs`:

- `impl AggregatePickerResult for MarketplacePickerItem { ... }`
- `impl TableDisplayable for MarketplacePickerItem { ... }`

2. Создать `Resource` и передавать его в универсальный пикер (см. Этап 2).

3. Выбор механизма в месте вызова:

```rust
let initial_id = form.get().marketplace_id.clone();
if cfg!(feature = "mp_special_picker") { // или условная компиляция/флаг
    open_specialized_picker(&modal, initial_id, on_confirm, on_cancel);
} else {
    open_universal_picker::<MarketplacePickerItem>(&modal, items_resource, initial_id, on_confirm, on_cancel);
}
```

---

## Этап 5. Встраивание в форму `ConnectionMPDetails`

1. Удалить старую разметку модалки и локальный `show_picker` из `crates/frontend/src/domain/a006_connection_mp/ui/details/view.rs`.

2. Подключить модальный сервис:

- `let modal = use_context::<ModalService>().expect("modal");`

3. В обработчике «Выбрать» открыть модалку:

```rust
modal.show(view! {
    <GenericAggregatePicker<MarketplacePickerItem>
        items=items_resource
        initial_selected_id=form.get().marketplace_id.clone()
        on_confirm=move |selected| { /* установить form.marketplace_id и marketplace_name; modal.hide() */ }
        on_cancel=move |_| modal.hide()
        render_row=None
    />
}.into_view());
```

4. В `on_confirm` записать выбранный `id` и «человекочитаемое» имя в поля формы и закрыть модалку.

---

## Этап 6. UX-улучшения (по мере необходимости)

- Индикация загрузки, сообщение об ошибке с кнопкой «Повторить».
- Поиск/фильтрация (клиентская, затем при необходимости серверная) с дебаунсом.
- «Очистить выбор» (кнопка сброса к `None`).
- Автоскролл к предвыбранной строке при открытии.

---

## Этап 7. Чек-лист тестирования

- Открытие пикера с уже установленным значением — строка подсвечена; клавиши стрелок меняют выделение; Enter подтверждает; Esc отменяет.
- Двойной клик по строке — немедленное подтверждение, форма получает `id` и отображаемое имя.
- Ошибка сети — отображается сообщение и «Повторить»; после повтора — успешная загрузка.
- Пустой список — корректно отображается пустое состояние.

---

## Что упрощено по сравнению с первой версией

- Нет `loader: async fn` в пропсах компонента — используется готовый `Resource` (меньше проблем с типами `Future` и временем жизни).
- Убран `Send` из колбэков — совместимо с WASM.
- Нет лишнего трэйта `PickableAggregate` — прямой вызов универсального пикера.
- Вся модальная логика централизована через `ModalService` и единый `ModalRenderer`.

---

## Пример минимальных сигнатур (для справки)

```rust
// layout/modal_service.rs
pub struct ModalService { /* show(View), hide() */ }
#[component] pub fn ModalProvider(children: Children) -> impl IntoView { /* ... */ }
#[component] pub fn ModalRenderer() -> impl IntoView { /* ... */ }

// shared/aggregate_picker.rs
pub trait AggregatePickerResult { fn id(&self) -> String; fn display_name(&self) -> String; }
pub trait TableDisplayable: AggregatePickerResult { fn code(&self) -> String; fn description(&self) -> String; }
#[component] pub fn GenericAggregatePicker<T>(/* см. Этап 3 */) -> impl IntoView where T: TableDisplayable + Clone + 'static { /* ... */ }
```

Этот план обеспечивает: предвыбор текущего значения, простоту внедрения, надёжный показ модалок и повторное использование для любых агрегатов.
