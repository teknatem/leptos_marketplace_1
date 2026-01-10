# Стандарт модальных окон и форм (Modal & Forms UI Standard)

## Обзор

Единый стандарт для всех модальных окон и форм в системе.

**Приоритет**: Thaw-first + hybrid
- По возможности используем **компоненты Thaw UI** и их стили.
- Там, где Thaw не покрывает кейс/нужна точная разметка, используем **наши core-классы** из `crates/frontend/static/themes/core/components.css` (например `.modal-*`, `.form__*`, `.button`, `.card`).

---

## Эталонные файлы

| Компонент           | Файл                                                          | Описание                    |
| ------------------- | ------------------------------------------------------------- | --------------------------- |
| CSS entrypoint      | `crates/frontend/index.html`                                  | Подключает core + theme CSS |
| Core CSS слои       | `crates/frontend/static/themes/core/index.css`                | tokens/base/layout/components/utilities |
| Стили модальных/форм| `crates/frontend/static/themes/core/components.css`           | `.modal-*`, `.form__*`, `.button`, `.card` |
| Dark theme modals   | `crates/frontend/static/themes/dark/dark.css`                 | Темная тема (строки 275-298)|
| Light theme modals  | `crates/frontend/static/themes/light/light.css`               | Светлая тема (строки 277-300)|
| Modal stack         | `crates/frontend/src/shared/modal_stack/mod.rs`               | `ModalStackService`, `ModalHost` |
| Modal frame         | `crates/frontend/src/shared/modal_frame/mod.rs`               | `ModalFrame` (overlay + surface) |

---

## Архитектура Modal

### Структура (актуально)

```rust
use crate::shared::modal_stack::ModalStackService;
use leptos::prelude::*;
use thaw::*;

let modal_stack =
    use_context::<ModalStackService>().expect("ModalStackService not found in context");

let open_modal = move |_| {
    modal_stack.push_with_frame(
        Some("max-width: 980px; width: min(980px, calc(100vw - 48px));".to_string()),
        None,
        move |handle| {
            view! {
                // ModalFrame уже рисует overlay + surface `.modal`.
                // Здесь — содержимое модалки (header+body по необходимости).
                <div class="modal-header modal-header--compact">
                    <h2 class="modal-title">"Заголовок"</h2>
                    <div class="modal-header-actions">
                        <Button appearance=ButtonAppearance::Primary on_click=move |_| {/* save */}>
                            "Сохранить"
                        </Button>
                        <Button appearance=ButtonAppearance::Transparent on_click=move |_| handle.close()>
                            "Закрыть"
                        </Button>
                    </div>
                </div>
                <div class="modal-body">
                    // ... form fields ...
                </div>
            }.into_any()
        },
    );
};
```

**Ключевые правила:**
1. **Модалки открываем через `ModalStackService`** (централизованный стек, Escape закрывает верхнюю).
2. `ModalFrame` рисует только overlay+surface. **Header/кнопки — внутри контента** (или в header detail-экрана).
3. Для UI — **Thaw-first**, а для кастомной разметки используем `.modal-*` и `.form__*` из core CSS.

### HTML Структура Modal (обновлено 2025-12-10)

```html
<div class="modal-overlay">                <!-- Полупрозрачный фон -->
  <div class="modal">                       <!-- Само модальное окно -->
    <div class="modal-header">              <!-- Заголовок + кнопки действий + кнопка X -->
      <h2 class="modal-title">Title</h2>
      <div class="modal-header-actions">    <!-- Контейнер для кнопок -->
        <!-- Предпочтительно Thaw Button, но допустимы `.button ...` -->
        <button class="button button--primary">Save</button>
        <button class="button button--secondary">Cancel</button>
      </div>
    </div>
    <div class="modal-body">                <!-- Содержимое (только поля формы) -->
      <!-- Поля формы здесь, БЕЗ кнопок -->
    </div>
  </div>
</div>
```

**Важные классы:**
- `.modal-header-actions` - контейнер для кнопок (flexbox с gap)

---

## Правила для форм

### 🎯 КРИТИЧЕСКОЕ ПРАВИЛО: Расположение кнопок

**Кнопки действий ВСЕГДА располагаются ВВЕРХУ формы (в modal-header), НЕ внизу!**

#### ❌ Неправильно (старый подход):
```rust
view! {
    <div class="modal-body">
        <form>
            <div class="form__group">...</div>
            <div class="form__group">...</div>
        </form>
    </div>
    <div class="form-actions">  <!-- Кнопки ВНИЗУ - НЕПРАВИЛЬНО -->
        <button>Save</button>
        <button>Cancel</button>
    </div>
}
```

#### ✅ Правильно (новый стандарт):
```rust
view! {
    <div class="modal-header modal-header--compact">
        <h2 class="modal-title">"Edit Item"</h2>
        <div class="modal-header-actions">
            <Button appearance=ButtonAppearance::Primary on_click=move |_| handle_save(())>
                "Сохранить"
            </Button>
            <Button appearance=ButtonAppearance::Transparent on_click=move |_| handle.close()>
                "Закрыть"
            </Button>
        </div>
    </div>
    <div class="modal-body">
        <div class="form__group">...</div>
        <div class="form__group">...</div>
    </div>
}
```

**Обоснование:**
- Кнопки видны сразу при открытии модального окна
- Не нужно скроллить вниз при длинных формах
- Следует паттерну из bolt-mpi-ui-redesign
- Улучшенный UX - быстрый доступ к действиям

### 🎯 ПРАВИЛО: Текст кнопки сохранения

**Кнопка сохранения должна называться "Save" (или "Сохранить" для русского интерфейса)**

- ✅ "Save" / "Сохранить"
- ❌ "Update" / "Обновить" 
- ❌ "Create" / "Создать"

Кнопка должна иметь единое название независимо от режима (создание/редактирование).

---

## Стили инпутов

### Размеры и классы (актуально)

**ВАЖНО:** если вы используете raw HTML элементы (не Thaw-компоненты), используйте core-классы:
`form__group`, `form__label`, `form__input`, `form__select`, `form__textarea`.

Размеры стандарта: **height 30px**, **padding 5px 12px** (см. `crates/frontend/static/themes/core/components.css`).

```html
<div class="form__group">
    <label class="form__label" for="field">Label</label>
    <input class="form__input" type="text" id="field" />
</div>

<div class="form__group">
    <label class="form__label" for="select">Select</label>
    <select class="form__select" id="select">...</select>
</div>

<div class="form__group">
    <label class="form__label" for="textarea">Textarea</label>
    <textarea class="form__textarea" id="textarea"></textarea>
</div>
```

**CSS стили:** см. `crates/frontend/static/themes/core/components.css` (секция Forms).


### Чекбоксы

Предпочтительно Thaw `Checkbox`. Для raw HTML используйте `.form__checkbox-wrapper`, `.form__checkbox`, `.form__checkbox-label` (см. `core/components.css`).

### Select (dropdown)

Для raw HTML используйте `.form__select` (уже включает `appearance` и стрелку через `--form-select-arrow`).

---

## Glassmorphism эффекты

### Modal overlay

```css
.modal-overlay {
    background-color: rgba(0, 0, 0, 0.5);  /* Полупрозрачный черный */
}
```

### Modal (light theme)

```css
.modal {
    background: rgba(255, 255, 255, 0.92);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(200, 200, 200, 0.5);
    backdrop-filter: blur(15px);
}
```

### Modal (dark theme)

```css
.modal {
    background: rgba(30, 41, 59, 0.85);
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
    border: 1px solid rgba(255, 255, 255, 0.2);
    backdrop-filter: blur(30px);
}
```

---

## Контрастность текста

### Dark Theme - Labels

Labels берут цвет из CSS переменных (например `--form-label-text`). Если контраст плохой — правим **переменные темы** в `crates/frontend/static/themes/{theme}/{theme}.css`, а не добавляем hardcode-цвета в компоненты.

### Light Theme - Labels

Аналогично: цвет лейблов должен определяться переменными темы.

---

## Анимации

```css
@keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
}

@keyframes slideUp {
    from {
        transform: translateY(20px);
        opacity: 0;
    }
    to {
        transform: translateY(0);
        opacity: 1;
    }
}

.modal-overlay {
    animation: fadeIn 0.2s ease;
}

.modal {
    animation: slideUp 0.2s ease;
}
```

---

## Функциональность

### Обязательные возможности

1. **Закрытие по Escape** - реализовано в Modal компоненте
2. **Закрытие по клику на overlay** - реализовано
3. **Кнопка X в header** - обязательна
4. **Prevent propagation на modal** - предотвращает закрытие при клике внутри

### Обработчики

```rust
// В list компоненте
let (show_modal, set_show_modal) = signal(false);
let (editing_id, set_editing_id) = signal::<Option<String>>(None);

let handle_create_new = move || {
    set_editing_id.set(None);
    set_show_modal.set(true);
};

let handle_edit = move |id: String| {
    set_editing_id.set(Some(id));
    set_show_modal.set(true);
};
```

---

## Миграция существующих модальных окон

### Шаги рефакторинга

1. Убедиться, что в корне приложения смонтирован `ModalHost` и предоставлен `ModalStackService` (см. `crates/frontend/src/app.rs`).\n
2. В месте открытия модалки получить сервис: `use_context::<ModalStackService>()`.\n
3. Открывать модалку через `modal_stack.push_with_frame(...)`.\n
4. Внутри контента использовать `.modal-header/.modal-body` и Thaw Buttons.\n
5. Удалить legacy-классы `btn`, `form-group` и устаревшие ссылки на `styles/3-components/*`.\n

---

## Рефакторированные модули (выполнено 2025-12-10)

✅ Следующие модули уже используют новый Modal компонент:

- `a001_connection_1c` - 1C соединения
- `a002_organization` - Организации  
- `a005_marketplace` - Маркетплейсы
- `a006_connection_mp` - Соединения с маркетплейсами
- `a007_marketplace_product` - Товары маркетплейсов

**TODO**: Остальные модальные окна (users, projections, etc.) нужно рефакторить по этому же стандарту.

---

## Чеклист для новых модальных окон (обновлено 2025-12-10)

- [ ] Использует `ModalStackService` + `ModalFrame` (через `push_with_frame`)
- [ ] Кнопки действий размещены вверху формы (перед полями)
- [ ] Кнопка сохранения называется "Сохранить"/"Save" (единое имя)
- [ ] Title динамический (Edit/New)
- [ ] Escape закрывает верхнюю модалку (через `ModalHost`)
- [ ] Закрытие по overlay работает (через `ModalFrame`, по умолчанию включено)
- [ ] Формы: Thaw-first; для raw HTML — `.form__*` классы
- [ ] Labels контрастные в темной теме

---

## Источник дизайна

Проект **bolt-mpi-ui-redesign** (`E:\\dev\\bolt\\bolt-mpi-ui-redesign`) остаётся визуальным референсом.\n
Источник истины по CSS в этом репозитории:\n
- `crates/frontend/static/themes/core/index.css`\n
- `crates/frontend/static/themes/core/components.css`\n
- `crates/frontend/static/themes/{theme}/{theme}.css`\n

**Ключевые размеры из bolt:**
- Input height: 30px
- Input padding: 5px 12px
- Form group margin-bottom: var(--spacing-md) = 16px
- Border radius: var(--border-radius-md) = 6px

