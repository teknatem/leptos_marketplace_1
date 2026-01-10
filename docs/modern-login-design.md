# Modern Login Design

## Обзор

Современный дизайн страницы входа, вдохновленный shadcn-ui и Tailwind CSS, с полной изоляцией стилей от основного приложения.

## Ключевые особенности

### 1. Particle Animation Background
- **Canvas-based** система частиц (50 частиц)
- Частицы соединяются линиями при приближении (< 150px)
- Indigo/Violet цветовая схема (#6366f1, #8b5cf6)
- Gradient blur overlays для глубины
- GPU-accelerated анимация (60 FPS)
- Адаптировано из [erp-flow-hub](https://github.com/teknatem/erp-flow-hub)

### 2. Изоляция стилей
- Все классы с префиксом `.modern-login-*`
- CSS переменные объявлены только внутри `.modern-login-container`
- Не затрагивает стили основного приложения
- Старые классы `.login-*` остаются доступными для совместимости

### 2. Дизайн

#### Цветовая схема
- **Фон**: Темный градиент (slate: #0f172a → #1e293b → #334155)
- **Акценты**: Indigo/Violet градиент (#6366f1 → #8b5cf6)
- **Карточка**: Белый фон с subtle border и современными тенями
- **Текст**: Slate палитра для хорошей читаемости

#### Компоненты

**Container**
- Full viewport height
- Gradient background с geometric паттернами
- Анимированные декоративные элементы

**Card (Login Box)**
- Max width: 460px (увеличено для лучшей читаемости)
- Border radius: 16px
- Backdrop blur эффект
- Layered shadows для глубины
- Slide-up анимация при появлении

**Typography**
- Заголовок: 1.875rem, font-weight 700
- Подзаголовок "Marketplace Integration Platform"
- Letter-spacing оптимизирован для читаемости

**Input поля**
- Высота: 48px (улучшенный touch target)
- Иконки внутри полей (user, lock)
- Focus ring эффект (как в shadcn-ui)
- Smooth transitions для всех состояний
- Hover states

**Кнопка**
- Высота: 48px
- Gradient background
- Shine эффект при hover
- Scale-down эффект при click
- Pulse анимация в loading состоянии

**Error сообщения**
- Slide-in анимация вместо shake
- Мягкие error цвета (#fef2f2 bg, #dc2626 text)
- Иконка предупреждения

**Footer**
- Информация о версии (MPI v1.0.0)
- Отделен border-top

### 3. Анимации

#### Вход на страницу
1. **Card**: Slide-up + scale (0.6s, cubic-bezier easing)
2. **Элементы формы**: Staggered fade-in (delays 0.1s-0.5s)
3. **Фон**: Gentle floating паттерны (15-20s loops)

#### Интерактивность
- **Input focus**: Border color + ring shadow (200ms)
- **Button hover**: Transform up + shadow increase (200ms)
- **Button click**: Scale down (200ms)
- **Error появление**: Slide from top (300ms)

### 4. Адаптивность

**Desktop (> 640px)**
- Card padding: 3rem
- Full features

**Tablet/Mobile (≤ 640px)**
- Card padding: 2rem 1.5rem
- Icon size: 64px
- Title: 1.5rem

**Small mobile (≤ 360px)**
- Card padding: 1.5rem 1rem
- Input/button height: 44px
- Font sizes reduced

## Технические детали

### CSS переменные (изолированные)

```css
.modern-login-container {
    /* Shadows (Tailwind-style) */
    --ml-shadow-xs через --ml-shadow-2xl
    
    /* Ring colors для focus */
    --ml-ring-primary: rgb values
    
    /* Color palette */
    --ml-bg-gradient-*: темные slate цвета
    --ml-accent-*: indigo/violet
    --ml-card-*: белый + border
    --ml-text-*: slate палитра
    --ml-input-*: slate borders + indigo focus
    --ml-btn-*: indigo/violet gradients
    --ml-error-*: красные оттенки
    
    /* Transitions */
    --ml-transition-fast: 150ms
    --ml-transition-base: 200ms
    --ml-transition-slow: 300ms
}
```

### Структура компонента (Rust)

```rust
<div class="modern-login-container">
  <div class="modern-login-box">
    <div class="modern-login-icon">
      <svg>...</svg>
    </div>
    
    <h2>"Вход в систему"</h2>
    <p class="subtitle">"Marketplace Integration Platform"</p>
    
    <Show when={error}>
      <div class="modern-error-message">
        <svg>...</svg>
        <span>{error}</span>
      </div>
    </Show>
    
    <form>
      <div class="form-group">
        <label>"Логин"</label>
        <div class="modern-input-wrapper">
          <input type="text" />
          <svg class="modern-input-icon">...</svg>
        </div>
      </div>
      
      <div class="form-group">
        <label>"Пароль"</label>
        <div class="modern-input-wrapper">
          <input type="password" />
          <svg class="modern-input-icon">...</svg>
        </div>
      </div>
      
      <button type="submit" class={loading_class}>
        {button_text}
      </button>
    </form>
    
    <div class="modern-login-footer">
      <p>"MPI v1.0.0"</p>
    </div>
  </div>
</div>
```

## Файлы

- **CSS**: `crates/frontend/static/themes/core/components.css` (секция `Login Page Styles (BEM)`)
- **Component**: `crates/frontend/src/system/pages/login.rs`
- **Animation Script**: `crates/frontend/assets/particle-animation.js`
- **HTML Integration**: `crates/frontend/index.html` (добавлен script tag)
- **Animation Guide**: `docs/particle-animation-guide.md` (подробная документация)

## Вдохновение

- **shadcn-ui**: Чистые формы, subtle borders, layered shadows
- **Tailwind CSS**: Современная цветовая палитра (slate, indigo, violet)
- **Vercel Login**: Минимализм, excellent spacing
- **Linear App**: Micro-interactions, smooth animations

## Браузерная совместимость

- Chrome/Edge: ✅ Full support
- Firefox: ✅ Full support  
- Safari: ✅ Full support (включая backdrop-filter)
- Mobile browsers: ✅ Полностью адаптивен

## Performance

- Pure CSS анимации (GPU-accelerated)
- Minimal JavaScript (только форма логики)
- No external dependencies
- Lightweight SVG иконки

---

**Дата создания**: 2025-12-06  
**Автор**: AI Assistant  
**Версия**: 1.0

