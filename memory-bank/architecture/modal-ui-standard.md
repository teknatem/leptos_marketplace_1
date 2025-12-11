# Стандарт модальных окон и форм (Modal & Forms UI Standard)

## Обзор

Единый стандарт для всех модальных окон и форм в системе. Основан на glassmorphism дизайне из bolt-mpi-ui-redesign.

---

## Эталонные файлы

| Компонент           | Файл                                                          | Описание                    |
| ------------------- | ------------------------------------------------------------- | --------------------------- |
| Modal компонент     | `crates/frontend/src/shared/modal/mod.rs`                     | Переиспользуемый компонент  |
| Стили модальных     | `crates/frontend/styles/3-components/modals.css`              | Базовые стили               |
| Стили форм          | `crates/frontend/styles/3-components/forms.css`               | Инпуты, labels, buttons     |
| Dark theme modals   | `crates/frontend/static/themes/dark/dark.css`                 | Темная тема (строки 275-298)|
| Light theme modals  | `crates/frontend/static/themes/light/light.css`               | Светлая тема (строки 277-300)|
| Эталон использования| `crates/frontend/src/domain/a001_connection_1c/ui/list/mod.rs`| Connection1C modal          |

---

## Архитектура Modal

### Структура компонента (обновлено 2025-12-10)

```rust
use crate::shared::modal::Modal;
use crate::shared::icons::icon;

<Show when=move || show_modal.get()>
    {move || {
        let modal_title = if editing_id.get().is_some() { 
            "Edit Item".to_string() 
        } else { 
            "New Item".to_string() 
        };
        
        view! {
            <Modal
                title=modal_title
                on_close=Callback::new(move |_| {
                    set_show_modal.set(false);
                    set_editing_id.set(None);
                })
                action_buttons=move || view! {
                    <button 
                        class="btn btn-primary" 
                        on:click=handle_save
                        disabled=move || !is_valid()
                    >
                        {icon("save")}
                        "Save"
                    </button>
                    <button 
                        class="btn btn-secondary" 
                        on:click=move |_| {
                            set_show_modal.set(false);
                            set_editing_id.set(None);
                        }
                    >
                        {icon("x")}
                        "Cancel"
                    </button>
                }
            >
                <!-- Только поля формы, БЕЗ кнопок -->
                <YourDetailsForm
                    id=editing_id.get()
                />
            </Modal>
        }
    }}
</Show>
```

**Ключевые изменения:**
1. Кнопки действий передаются через параметр `action_buttons`
2. Кнопки размещаются в header (справа от заголовка, слева от X)
3. Кнопка сохранения всегда называется "Save"
4. Кнопка закрытия (X) всегда крайняя справа

### HTML Структура Modal (обновлено 2025-12-10)

```html
<div class="modal-overlay">                <!-- Полупрозрачный фон -->
  <div class="modal">                       <!-- Само модальное окно -->
    <div class="modal-header">              <!-- Заголовок + кнопки действий + кнопка X -->
      <h2 class="modal-title">Title</h2>
      <div class="modal-header-actions">    <!-- Контейнер для кнопок -->
        <button class="btn btn-primary">Save</button>
        <button class="btn btn-secondary">Cancel</button>
        <button class="btn btn-ghost btn-close">X</button>
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
- `.btn-close` - кнопка закрытия (всегда последняя)

---

## Правила для форм

### 🎯 КРИТИЧЕСКОЕ ПРАВИЛО: Расположение кнопок

**Кнопки действий ВСЕГДА располагаются ВВЕРХУ формы (в modal-header), НЕ внизу!**

#### ❌ Неправильно (старый подход):
```rust
view! {
    <div class="modal-body">
        <form>
            <div class="form-group">...</div>
            <div class="form-group">...</div>
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
    <Modal 
        title="Edit Item"
        on_close=...
        action_buttons=move || view! {
            <button class="btn btn-primary" on:click=handle_save>
                {icon("save")}
                "Save"
            </button>
            <button class="btn btn-secondary" on:click=handle_cancel>
                {icon("x")}
                "Cancel"
            </button>
        }
    >
        <!-- Только поля формы в body -->
        <div class="form-group">...</div>
        <div class="form-group">...</div>
    </Modal>
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

### Размеры и классы (из bolt-mpi-ui-redesign, обновлено 2025-12-10)

**ВАЖНО: Используйте классы из bolt проекта!**

```html
<div class="form-group">
    <label class="form-label" for="field">Label</label>
    <input class="form-input" type="text" id="field" />
</div>

<div class="form-group">
    <label class="form-label" for="select">Select</label>
    <select class="form-select" id="select">...</select>
</div>

<div class="form-group">
    <label class="form-label" for="textarea">Textarea</label>
    <textarea class="form-textarea" id="textarea"></textarea>
</div>
```

**CSS стили (автоматически применяются при использовании классов):**

```css
.form-input,
.form-select,
.form-textarea {
    width: 100%;
    padding: 5px 12px;           /* Из bolt-mpi-ui-redesign */
    height: 30px;                /* Из bolt-mpi-ui-redesign */
    font-size: var(--font-size-base);  /* 13px */
    line-height: 1.4;
    border-radius: var(--radius-md);   /* 6px */
}

.form-textarea {
    height: auto;
    min-height: 60px;
}

.form-label {
    display: block;
    margin-bottom: 4px;
    font-size: var(--font-size-sm);  /* 12px */
    font-weight: 500;
}
```


### Чекбоксы

```css
.checkbox-label {
    display: flex;
    align-items: center;
    gap: var(--spacing-md);     /* Минимум 8-12px между чекбоксом и текстом */
    cursor: pointer;
    font-weight: normal !important;
}

.checkbox-group {
    display: flex;
    flex-direction: column;
    gap: var(--spacing-sm);     /* Интервал между чекбоксами */
}
```

**ВАЖНО**: Чекбокс и его label должны иметь gap минимум 8-12px для визуального разделения.

### Select (dropdown)

```css
.form-group select {
    appearance: none;
    background-position: right 10px center;
    background-size: 1em;
    padding-right: 32px;  /* Место для стрелки */
}
```

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

**КРИТИЧЕСКИ ВАЖНО**: Labels должны быть яркими в темной теме!

```css
/* Dark theme */
.form-label,
.form-group label {
    color: rgba(255, 255, 255, 0.95) !important;
}
```

### Light Theme - Labels

```css
.form-label,
.form-group label {
    color: var(--color-neutral-800);
}
```

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

1. Добавить импорт Modal:
```rust
use crate::shared::modal::Modal;
```

2. Заменить старую структуру на новую:
```rust
// Было
<div class="modal-overlay">
    <div class="modal-content">
        <YourDetails ... />
    </div>
</div>

// Стало
<Show when=move || show_modal.get()>
    {move || {
        let modal_title = if editing_id.get().is_some() { "Edit".to_string() } else { "New".to_string() };
        view! {
            <Modal title=modal_title on_close=...>
                <YourDetails ... />
            </Modal>
        }
    }}
</Show>
```

3. Удалить неиспользуемые `handle_cancel` closures

4. Переместить кнопки действий в header (НОВОЕ ПРАВИЛО)

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

- [ ] Использует `crate::shared::modal::Modal` компонент
- [ ] Кнопки действий размещены вверху формы (перед полями)
- [ ] Кнопка сохранения называется "Save" (не "Update", не "Create")
- [ ] Title динамический (Edit/New)
- [ ] Обработчик on_close корректно закрывает модал
- [ ] Используется `<Show when=...>` для условного рендеринга
- [ ] Поддержка Escape (автоматически через Modal)
- [ ] Закрытие по overlay (автоматически через Modal)
- [ ] **ВАЖНО: Используются классы из bolt:**
  - [ ] `class="form-group"` для контейнера поля
  - [ ] `class="form-label"` для label
  - [ ] `class="form-input"` для input
  - [ ] `class="form-select"` для select
  - [ ] `class="form-textarea"` для textarea
- [ ] Правильные стили (автоматически через классы: height: 30px, padding: 5px 12px)
- [ ] Чекбоксы используют `class="checkbox-label"` с gap
- [ ] Labels контрастные в темной теме

---

## Источник дизайна

Стили взяты из проекта **bolt-mpi-ui-redesign** (`E:\dev\bolt\bolt-mpi-ui-redesign`):
- `src\components\Modal.tsx` - React компонент модального окна
- `public\themes\base.css` - базовые стили (form-input: padding 5px 12px, height 30px)
- `public\themes\dark\dark.css` - темная тема
- `public\themes\light\light.css` - светлая тема

**Ключевые размеры из bolt:**
- Input height: 30px
- Input padding: 5px 12px
- Form group margin-bottom: var(--spacing-md) = 16px
- Border radius: var(--border-radius-md) = 6px

