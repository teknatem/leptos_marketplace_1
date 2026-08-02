# Быстрое исправление ошибки config.toml

## Проблема

При запуске backend.exe вы видите ошибку:

```
✗ ERROR: Invalid TOML format: TOML parse error at line 7, column 13
invalid unicode 8-digit hex code
```

## Причина

В файле `config.toml` используются обратные слеши Windows `\` в путях, что вызывает ошибку парсинга TOML.

## Решение (30 секунд)

1. Откройте файл `C:\Users\udv\Desktop\MPI\config.toml` в Блокноте
2. Найдите строку с путем к базе данных:
   ```toml
   path = "C:\Users\udv\Desktop\MPI\data\app.db"
   ```
3. Замените **обратные** слеши `\` на **прямые** слеши `/`:
   ```toml
   path = "C:/Users/udv/Desktop/MPI/data/app.db"
   ```
4. Сохраните файл
5. Запустите backend.exe снова

## Готовый пример config.toml

Скопируйте это содержимое в ваш `config.toml`:

```toml
# Marketplace Integrator Configuration
[database]
path = "C:/Users/udv/Desktop/MPI/data/app.db"
```

**Важно:** Используйте прямые слеши `/` - Windows их полностью поддерживает!

## Почему это происходит?

TOML (формат конфигурационных файлов) интерпретирует обратный слеш `\` как начало escape-последовательности (как `\n` для новой строки). Когда он видит `\U`, он думает, что это Unicode код и ожидает 8 цифр после него.

Windows поддерживает оба типа слешей в путях, поэтому использование прямых слешей `/` - самое простое решение.

## Альтернативные варианты (если прямые слеши не подходят)

### Вариант 1: Удвоить обратные слеши

```toml
path = "C:\\Users\\udv\\Desktop\\MPI\\data\\app.db"
```

### Вариант 2: Использовать одинарные кавычки

```toml
path = 'C:\Users\udv\Desktop\MPI\data\app.db'
```

### Вариант 3: Использовать относительный путь

```toml
path = "data/app.db"
```

(База данных будет создана в папке `C:\Users\udv\Desktop\MPI\data\`)

## Проверка

После исправления при запуске вы должны увидеть:

```
✓ TOML parsed successfully
✓ Database path from config: C:/Users/udv/Desktop/MPI/data/app.db
✓ Configuration loaded successfully!
```

Затем программа продолжит загрузку и запустится успешно.
