# Theme Persistence Implementation

## Описание проблемы

На экране логина не переключалась тема, и выбранная тема не сохранялась после перезагрузки страницы.

**Корневая причина:** CSS стили используют селектор `body[data-theme="..."]`, но атрибут `data-theme` не устанавливался на элементе `<body>`.

## Решение

Реализована система сохранения темы с использованием localStorage.

### Изменённые файлы

1. **`crates/frontend/src/shared/theme/theme_select.rs`**

   - Добавлены функции для работы с localStorage:
     - `get_saved_theme()` - загрузка сохранённой темы из localStorage
     - `save_theme()` - сохранение выбранной темы в localStorage
     - `apply_theme()` - применение темы к документу
   - **Ключевое изменение:** Функция `apply_theme()` теперь устанавливает атрибут `data-theme` на `<body>`:
     ```rust
     document.body().set_attribute("data-theme", theme);
     ```
   - При монтировании компонента загружается сохранённая тема (по умолчанию "dark")
   - При изменении темы она автоматически сохраняется в localStorage

2. **`crates/frontend/index.html`**
   - Добавлен inline-скрипт в начале `<body>`, который:
     - Загружает сохранённую тему из localStorage
     - **Устанавливает `data-theme` атрибут на body**
     - Обновляет theme stylesheet
   - Скрипт выполняется синхронно в начале body, предотвращая "мигание" темы

## Ключ хранения

Тема сохраняется в localStorage под ключом: `app_theme`

## Поддерживаемые темы

- `dark` - Тёмная (по умолчанию)
- `light` - Светлая
- `forest` - Лесная

## Как это работает

### Механизм применения темы

CSS стили для страницы логина используют селекторы типа:

```css
body[data-theme="dark"] .login__card {
  background: rgba(30, 41, 59, 0.95);
}
```

Поэтому критически важно устанавливать атрибут `data-theme` на элемент `<body>`.

### Поток работы

1. **При загрузке страницы (index.html):**

   - Inline-скрипт в начале `<body>` выполняется первым:
     ```javascript
     document.body.setAttribute("data-theme", savedTheme);
     ```
   - Загружает сохранённую тему из localStorage (по умолчанию "dark")
   - Устанавливает атрибут `data-theme` на body
   - Обновляет theme stylesheet link
   - Всё это происходит **до** загрузки WASM и Leptos компонентов

2. **В компоненте ThemeSelect (Rust/Leptos):**

   - При монтировании:
     - Читает сохранённую тему из localStorage
     - Вызывает `apply_theme()` через Effect
   - При переключении темы:
     - Вызывает `apply_theme()` для установки `data-theme` и обновления stylesheet
     - Сохраняет в localStorage
     - Обновляет UI (активная тема в dropdown)

3. **На экране логина:**
   - CSS стили реагируют на `body[data-theme="..."]` моментально
   - ThemeSelect компонент работает одинаково на всех экранах
   - Тема сохраняется между сессиями

## Тестирование

1. Откройте страницу логина
2. Переключите тему (например, на "Светлая")
3. Перезагрузите страницу - тема должна остаться светлой
4. Войдите в систему - тема должна сохраниться
5. Выйдите и снова зайдите - тема должна быть той же

## Технические детали

- **localStorage key:** `app_theme`
- **Default theme:** `dark`
- **Theme path format:** `static/themes/{theme}/{theme}.css`
- **Body attribute:** `data-theme="dark|light|forest"`
- **Effect hook:** Используется `Effect::new()` для применения темы при монтировании Leptos компонента
- **Inline script:** Выполняется синхронно в начале `<body>` для предотвращения FOUC (Flash of Unstyled Content)

## Эталонный проект

Реализация основана на подходе из `E:\dev\bolt\bolt-mpi-ui-redesign`:

- Используется атрибут `body[data-theme]` для CSS селекторов
- Тема загружается из localStorage при старте
- CSS файлы содержат переопределения для каждой темы типа `body[data-theme="dark"] .element { ... }`
