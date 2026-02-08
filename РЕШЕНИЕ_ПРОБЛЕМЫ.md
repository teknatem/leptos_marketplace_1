# ✅ Решение проблемы запуска на Windows Server

## Что было сделано

### 1. Добавлена подробная диагностика ✓

Теперь backend выводит детальную информацию о каждом шаге запуска:

- ✓ Инициализация системы логирования
- ✓ Загрузка конфигурации
- ✓ Инициализация базы данных
- ✓ Настройка маршрутов
- ✓ Запуск сервера

При любой ошибке выводятся:

- Точное описание проблемы
- Возможные причины
- Рекомендации по устранению

### 2. Обнаружена проблема в вашем config.toml ✓

**Проблема:** В файле используются обратные слеши Windows `\`, которые TOML интерпретирует как escape-последовательности.

**Ваш файл:**

```toml
path = "C:\Users\udv\Desktop\MPI\data\app.db"  ❌
       ↑ \U интерпретируется как Unicode код
```

## 🚀 Быстрое исправление (1 минута)

### Вариант 1: Замените слеши (рекомендуется)

1. Откройте `C:\Users\udv\Desktop\MPI\config.toml`
2. Измените строку:
   ```toml
   [database]
   path = "C:/Users/udv/Desktop/MPI/data/app.db"
   ```
   ↑ Замените `\` на `/`
3. Сохраните и запустите backend.exe

### Вариант 2: Используйте одинарные кавычки

```toml
[database]
path = 'C:\Users\udv\Desktop\MPI\data\app.db'
```

↑ Одинарные кавычки = literal string (слеши не обрабатываются)

### Вариант 3: Удвойте обратные слеши

```toml
[database]
path = "C:\\Users\\udv\\Desktop\\MPI\\data\\app.db"
```

↑ Каждый `\` становится `\\`

## 📋 Правильный config.toml для вашего сервера

Скопируйте это содержимое в файл `C:\Users\udv\Desktop\MPI\config.toml`:

```toml
# Marketplace Integrator Configuration

[database]
# Используйте прямые слеши - Windows их поддерживает!
path = "C:/Users/udv/Desktop/MPI/data/app.db"
```

## ✓ После исправления

При запуске backend.exe вы увидите:

```
╔══════════════════════════════════════════════════════════╗
║           MARKETPLACE BACKEND STARTING...               ║
╚══════════════════════════════════════════════════════════╝

Step 1: Initializing logging system...
✓ Logging system initialized

Step 2: Initializing database...
✓ Config file found!
✓ File read successfully
✓ TOML parsed successfully
✓ Database path from config: C:/Users/udv/Desktop/MPI/data/app.db
✓ Configuration loaded successfully!
✓ Database initialized successfully

Step 3: Applying authentication system migration...
✓ Auth migration completed

Step 4: Checking admin user...
✓ Admin user verified

Step 5: Initializing scheduled tasks...
✓ Scheduled tasks initialized

Step 6: Starting background worker...
✓ Background worker started

Step 7: Configuring CORS...
✓ CORS configured

Step 8: Building application routes...
✓ Routes configured

Step 9: Starting HTTP server...
✓ Server successfully bound to port 3000

╔══════════════════════════════════════════════════════════╗
║           SERVER STARTED SUCCESSFULLY!                  ║
║  Server listening on: http://0.0.0.0:3000              ║
║  Press Ctrl+C to stop                                   ║
╚══════════════════════════════════════════════════════════╝
```

## 📝 Дополнительные материалы

- **QUICK_FIX_CONFIG.md** - быстрая инструкция по исправлению
- **DIAGNOSTICS_GUIDE.md** - полное руководство по диагностике
- **config.toml.example** - пример правильного конфигурационного файла

## 💡 Почему прямые слеши работают на Windows?

Windows поддерживает оба типа разделителей пути:

- `C:\Users\...` (традиционный Windows)
- `C:/Users/...` (Unix-стиль, но работает на Windows!)

Большинство современных программ и API Windows принимают прямые слеши.
Это делает конфиги более универсальными и избавляет от проблем с экранированием.

## 🎯 Итого

1. ✅ Добавлена подробная диагностика процесса загрузки
2. ✅ Обнаружена проблема с обратными слешами в config.toml
3. ✅ Добавлена автоматическая подсказка при этой ошибке
4. ✅ Создана документация с решением

**Следующий шаг:** Исправьте config.toml и запустите backend.exe снова!
