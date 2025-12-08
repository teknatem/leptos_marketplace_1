# Particle Animation для Login Page

## Обзор

Фоновая анимация частиц на странице логина, адаптированная из проекта [erp-flow-hub](https://github.com/teknatem/erp-flow-hub).

## Что это такое?

**Canvas-based particle system** с интерактивными эффектами:
- 50 частиц, движущихся по экрану
- Частицы соединяются линиями, когда находятся близко друг к другу (< 150px)
- Плавные gradient overlays для глубины
- Цветовая схема: Indigo (#6366f1) и Violet (#8b5cf6)

## Файлы

### 1. JavaScript Animation
**Путь**: `crates/frontend/assets/particle-animation.js`

Чистый JavaScript скрипт, использующий Canvas API:
- Создает 50 частиц с случайными позициями и скоростями
- Анимирует частицы с помощью `requestAnimationFrame`
- Рисует соединительные линии между близкими частицами
- Автоматически подстраивается под размер окна

### 2. CSS Styles
**Путь**: `crates/frontend/styles/3-components/login.css`

Добавлен класс `.particle-canvas`:
```css
.particle-canvas {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    pointer-events: none; /* Не блокирует клики */
    z-index: 0;           /* Позади формы */
}
```

### 3. HTML Integration
**Путь**: `crates/frontend/index.html`

Скрипт загружается с атрибутом `defer`:
```html
<script src="assets/particle-animation.js" defer></script>
```

### 4. Leptos Component
**Путь**: `crates/frontend/src/system/pages/login.rs`

Canvas элемент добавлен в view:
```rust
<canvas id="particle-canvas" class="particle-canvas"></canvas>
```

## Как это работает

### 1. Инициализация
При загрузке страницы скрипт:
1. Находит canvas элемент по ID `particle-canvas`
2. Получает 2D context
3. Устанавливает размер canvas = размеру окна
4. Создает 50 частиц с случайными параметрами

### 2. Particle Object
Каждая частица имеет:
```javascript
{
    x: number,        // Позиция X
    y: number,        // Позиция Y
    vx: number,       // Скорость по X (-0.25 до 0.25)
    vy: number,       // Скорость по Y (-0.25 до 0.25)
    size: number,     // Размер (1-3px)
    opacity: number   // Прозрачность (0.2-0.7)
}
```

### 3. Animation Loop
На каждом кадре (60 FPS):
1. Рисует полупрозрачный слой фона (создает trailing эффект)
2. Обновляет позиции всех частиц
3. Проверяет границы и отражает частицы от краев
4. Рисует каждую частицу
5. Проверяет расстояния между частицами
6. Рисует соединительные линии между близкими частицами

### 4. Connection Algorithm
```javascript
if (distance < 150px) {
    lineOpacity = 0.15 * (1 - distance / 150)
    // Чем ближе частицы, тем ярче линия
}
```

## Performance

- **Canvas rendering**: GPU-accelerated
- **Particle count**: 50 (оптимально для производительности)
- **Frame rate**: ~60 FPS на современных устройствах
- **Memory**: ~1-2 MB
- **CPU**: Минимальная нагрузка благодаря `requestAnimationFrame`

## Customization

### Изменить количество частиц
В `particle-animation.js`, строка 21:
```javascript
const particleCount = 50; // Измените на нужное число
```

### Изменить цвет частиц
Строка 53 (индиго) или измените на свой:
```javascript
ctx.fillStyle = `rgba(99, 102, 241, ${particle.opacity})`;
//                    ^^^^^^^^^^^^^ - RGB цвет
```

**Примеры цветов:**
- Indigo (текущий): `99, 102, 241`
- Violet: `139, 92, 246`
- Green (как в erp-flow-hub): `76, 175, 80`
- Blue: `59, 130, 246`
- Cyan: `34, 211, 238`

### Изменить дистанцию соединения
Строка 22:
```javascript
const connectionDistance = 150; // px
```

### Изменить скорость частиц
Строки 29-30:
```javascript
vx: (Math.random() - 0.5) * 0.5,  // Умножьте на большее число для ускорения
vy: (Math.random() - 0.5) * 0.5,
```

### Изменить размер частиц
Строка 31:
```javascript
size: Math.random() * 2 + 1,  // От 1 до 3px
```

## Gradient Overlays

В дополнение к частицам, используются CSS псевдо-элементы для создания градиентных пятен:

```css
.modern-login-container::before {
    /* Левый верхний indigo blob */
    background: radial-gradient(circle, rgba(99, 102, 241, 0.15) 0%, transparent 70%);
    filter: blur(80px);
}

.modern-login-container::after {
    /* Правый нижний violet blob */
    background: radial-gradient(circle, rgba(139, 92, 246, 0.15) 0%, transparent 70%);
    filter: blur(80px);
}
```

Эти пятна медленно двигаются (20s и 15s анимации).

## Дополнительные возможности

### SVG Decorations (опционально)

В erp-flow-hub также используются SVG декоративные элементы:
- Вращающиеся шестеренки
- Circuit paths (пути схем)
- Data nodes с pulse анимацией
- Бинарный код
- Server/Database иконки

Если хотите добавить их, можно портировать `AutomationSVG.tsx` из erp-flow-hub.

## Browser Support

- ✅ Chrome/Edge 90+
- ✅ Firefox 88+
- ✅ Safari 14+
- ✅ Mobile browsers (iOS Safari, Chrome Mobile)

Canvas API поддерживается всеми современными браузерами.

## Troubleshooting

### Анимация не запускается
1. Проверьте консоль браузера на ошибки
2. Убедитесь, что `particle-animation.js` загружен (Network tab в DevTools)
3. Проверьте, что canvas существует в DOM (`<canvas id="particle-canvas">`)

### Низкая производительность
1. Уменьшите количество частиц (< 50)
2. Увеличьте `connectionDistance` (меньше проверок расстояний)
3. Упростите trailing эффект (увеличьте opacity фона)

### Canvas не на весь экран
1. Проверьте CSS `.particle-canvas` - должен быть `position: fixed`
2. Убедитесь, что `resizeCanvas()` вызывается при ресайзе окна

## Источник

Адаптировано из: https://github.com/teknatem/erp-flow-hub
- File: `src/components/login/AnimatedBackground.tsx`
- Оригинальный проект использовал зеленый цвет (RGB: 76, 175, 80)
- Адаптирован для indigo/violet палитры нашего проекта

---

**Дата**: 2025-12-06  
**Автор**: AI Assistant  
**Версия**: 1.0

