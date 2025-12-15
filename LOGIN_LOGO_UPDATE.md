# Login Logo Update

## Описание изменения

Заменён inline SVG логотип на странице логина на файл `gears.svg` из эталонного проекта `bolt-mpi-ui-redesign`.

## Изменённые файлы

### 1. Добавлен файл

- **`crates/frontend/assets/images/gears.svg`**
  - Скопирован из `E:\dev\bolt\bolt-mpi-ui-redesign\public\gears.svg`
  - Цветной SVG логотип с шестерёнками

### 2. Обновлён код

- **`crates/frontend/src/system/pages/login.rs`**

**Было:**

```rust
<div class="login__logo">
    <svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M12 2L2 7L12 12L22 7L12 2Z" fill="rgba(96, 165, 250, 0.2)" />
        <path stroke-linecap="round" stroke-linejoin="round" d="M2 17L12 22L22 17" />
        <path stroke-linecap="round" stroke-linejoin="round" d="M2 12L12 17L22 12" />
    </svg>
</div>
```

**Стало:**

```rust
<div class="login__logo">
    <img src="assets/images/gears.svg" alt="Integrator" width="64" height="64" />
</div>
```

## CSS стили

Существующие стили уже правильно настроены и соответствуют эталонному проекту:

```css
.login__logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  margin-bottom: var(--spacing-md);
}

.login__logo img {
  width: 64px;
  height: 64px;
  object-fit: contain;
}
```

## Преимущества

1. **Единообразие с эталонным проектом** - использует тот же логотип
2. **Лучшее качество** - цветной векторный логотип вместо простого контурного
3. **Удобство поддержки** - логотип можно легко заменить, не трогая код
4. **Кеширование** - браузер кеширует отдельный файл SVG

## Тестирование

Код успешно скомпилирован. Для проверки:

1. Перезапустите trunk serve (если нужно)
2. Откройте страницу логина
3. Логотип должен отображаться как цветные шестерёнки
4. Размер 64x64 пикселя
