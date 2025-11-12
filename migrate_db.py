#!/usr/bin/env python3
"""
Скрипт миграции таблицы p902_ozon_finance_realization
Добавляет колонку is_return и изменяет PRIMARY KEY
"""

import sqlite3
import sys
from datetime import datetime

DB_PATH = "target/db/app.db"

def main():
    print("=== P902 Migration Script (Python) ===\n")

    try:
        # Подключаемся к БД
        print(f"Connecting to database: {DB_PATH}")
        conn = sqlite3.connect(DB_PATH)
        cursor = conn.cursor()

        # Проверяем существование таблицы
        cursor.execute("SELECT name FROM sqlite_master WHERE type='table' AND name='p902_ozon_finance_realization';")
        table_exists = cursor.fetchone()

        if not table_exists:
            print("❌ Table p902_ozon_finance_realization does not exist")
            conn.close()
            return 1

        print("✅ Table exists")

        # Проверяем текущую схему
        cursor.execute("PRAGMA table_info(p902_ozon_finance_realization);")
        columns = cursor.fetchall()

        has_is_return = any(col[1] == 'is_return' for col in columns)

        if has_is_return:
            print("✅ Table already has is_return column - migration not needed")
            conn.close()
            return 0

        print("⚠️  Table needs migration\n")

        # Получаем количество записей
        cursor.execute("SELECT COUNT(*) FROM p902_ozon_finance_realization;")
        count = cursor.fetchone()[0]
        print(f"📊 Current records: {count}\n")

        if count > 0:
            response = input(f"⚠️  Found {count} existing records. Continue with migration? (yes/no): ")
            if response.lower() not in ['yes', 'y']:
                print("Migration cancelled")
                conn.close()
                return 0

        print("🔄 Starting migration...\n")

        # Начинаем транзакцию
        cursor.execute("BEGIN TRANSACTION;")

        try:
            # Шаг 1: Создаем новую таблицу
            print("1. Creating new table structure...")
            cursor.execute("""
                CREATE TABLE p902_ozon_finance_realization_new (
                    posting_number TEXT NOT NULL,
                    sku TEXT NOT NULL,
                    document_type TEXT NOT NULL,
                    registrator_ref TEXT NOT NULL,
                    connection_mp_ref TEXT NOT NULL,
                    organization_ref TEXT NOT NULL,
                    posting_ref TEXT,
                    accrual_date TEXT NOT NULL,
                    operation_date TEXT,
                    delivery_date TEXT,
                    delivery_schema TEXT,
                    delivery_region TEXT,
                    delivery_city TEXT,
                    quantity REAL NOT NULL,
                    price REAL,
                    amount REAL NOT NULL,
                    commission_amount REAL,
                    commission_percent REAL,
                    services_amount REAL,
                    payout_amount REAL,
                    operation_type TEXT NOT NULL,
                    operation_type_name TEXT,
                    is_return INTEGER NOT NULL DEFAULT 0,
                    currency_code TEXT,
                    loaded_at_utc TEXT NOT NULL,
                    payload_version INTEGER NOT NULL DEFAULT 1,
                    extra TEXT,
                    PRIMARY KEY (posting_number, sku, operation_type)
                );
            """)
            print("   ✅ New table created")

            # Шаг 2: Копируем данные
            print("2. Copying existing data...")
            cursor.execute("""
                INSERT INTO p902_ozon_finance_realization_new (
                    posting_number, sku, document_type, registrator_ref,
                    connection_mp_ref, organization_ref, posting_ref,
                    accrual_date, operation_date, delivery_date,
                    delivery_schema, delivery_region, delivery_city,
                    quantity, price, amount, commission_amount, commission_percent,
                    services_amount, payout_amount,
                    operation_type, operation_type_name, is_return,
                    currency_code, loaded_at_utc, payload_version, extra
                )
                SELECT
                    posting_number, sku, document_type, registrator_ref,
                    connection_mp_ref, organization_ref, posting_ref,
                    accrual_date, operation_date, delivery_date,
                    delivery_schema, delivery_region, delivery_city,
                    quantity, price, amount, commission_amount, commission_percent,
                    services_amount, payout_amount,
                    operation_type, operation_type_name, 0 as is_return,
                    currency_code, loaded_at_utc, payload_version, extra
                FROM p902_ozon_finance_realization;
            """)
            copied = cursor.rowcount
            print(f"   ✅ Copied {copied} records")

            # Шаг 3: Удаляем старую таблицу
            print("3. Dropping old table...")
            cursor.execute("DROP TABLE p902_ozon_finance_realization;")
            print("   ✅ Old table dropped")

            # Шаг 4: Переименовываем новую таблицу
            print("4. Renaming new table...")
            cursor.execute("ALTER TABLE p902_ozon_finance_realization_new RENAME TO p902_ozon_finance_realization;")
            print("   ✅ Table renamed")

            # Шаг 5: Создаем индексы
            print("5. Creating indexes...")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_p902_accrual_date ON p902_ozon_finance_realization (accrual_date);")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_p902_posting_number ON p902_ozon_finance_realization (posting_number);")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_p902_connection_mp_ref ON p902_ozon_finance_realization (connection_mp_ref);")
            cursor.execute("CREATE INDEX IF NOT EXISTS idx_p902_posting_ref ON p902_ozon_finance_realization (posting_ref);")
            print("   ✅ Indexes created")

            # Коммитим транзакцию
            conn.commit()

            print("\n✅ Migration completed successfully!")
            print(f"📊 Total records migrated: {copied}")

            # Проверяем результат
            cursor.execute("SELECT COUNT(*) FROM p902_ozon_finance_realization;")
            final_count = cursor.fetchone()[0]
            print(f"📊 Final record count: {final_count}")

            if final_count == count:
                print("✅ All records preserved")
            else:
                print(f"⚠️  Record count changed: {count} -> {final_count}")

        except Exception as e:
            conn.rollback()
            print(f"\n❌ Error during migration: {e}")
            print("🔄 Rolling back changes...")
            return 1

        finally:
            conn.close()

        print("\n✅ Done! You can now restart the backend.")
        return 0

    except sqlite3.Error as e:
        print(f"❌ Database error: {e}")
        return 1
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        return 1

if __name__ == "__main__":
    sys.exit(main())
