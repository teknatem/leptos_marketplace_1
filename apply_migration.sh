#!/bin/bash

# Скрипт для применения миграции p902_ozon_finance_realization

DB_PATH="target/db/app.db"
MIGRATION_SQL="migrate_p902.sql"

echo "=== P902 Migration Script ==="
echo ""

# Проверяем существование БД
if [ ! -f "$DB_PATH" ]; then
    echo "❌ Ошибка: База данных не найдена по пути: $DB_PATH"
    exit 1
fi

# Проверяем существование SQL скрипта
if [ ! -f "$MIGRATION_SQL" ]; then
    echo "❌ Ошибка: SQL миграция не найдена: $MIGRATION_SQL"
    exit 1
fi

# Проверяем доступность sqlite3
if ! command -v sqlite3 &> /dev/null; then
    echo "❌ Ошибка: sqlite3 не установлен"
    echo "Установите: sudo apt install sqlite3 (Linux) или brew install sqlite3 (macOS)"
    exit 1
fi

echo "✅ База данных найдена: $DB_PATH"
echo "✅ SQL миграция найдена: $MIGRATION_SQL"
echo ""

# Показываем текущее количество записей
CURRENT_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM p902_ozon_finance_realization;" 2>/dev/null || echo "0")
echo "📊 Текущее количество записей в p902: $CURRENT_COUNT"
echo ""

# Создаем backup
BACKUP_PATH="$DB_PATH.backup_$(date +%Y%m%d_%H%M%S)"
echo "💾 Создаем backup: $BACKUP_PATH"
cp "$DB_PATH" "$BACKUP_PATH"

if [ $? -eq 0 ]; then
    echo "✅ Backup создан успешно"
else
    echo "❌ Ошибка создания backup"
    exit 1
fi

echo ""
echo "🔄 Применяем миграцию..."
echo ""

# Применяем миграцию
sqlite3 "$DB_PATH" < "$MIGRATION_SQL"

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Миграция применена успешно!"

    # Проверяем новую схему
    NEW_COUNT=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM p902_ozon_finance_realization;")
    echo "📊 Количество записей после миграции: $NEW_COUNT"

    if [ "$CURRENT_COUNT" -eq "$NEW_COUNT" ]; then
        echo "✅ Все данные сохранены"
    else
        echo "⚠️ Внимание: количество записей изменилось!"
    fi

    echo ""
    echo "✅ Готово! Теперь можно запускать backend."
    echo "💾 Backup сохранен: $BACKUP_PATH"
else
    echo ""
    echo "❌ Ошибка при применении миграции"
    echo "💾 Восстанавливаем из backup..."
    cp "$BACKUP_PATH" "$DB_PATH"
    echo "✅ База данных восстановлена из backup"
    exit 1
fi
