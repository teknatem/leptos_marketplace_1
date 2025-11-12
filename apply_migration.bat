@echo off
REM Скрипт для применения миграции p902_ozon_finance_realization (Windows)

set DB_PATH=target\db\app.db
set MIGRATION_SQL=migrate_p902.sql

echo === P902 Migration Script ===
echo.

REM Проверяем существование БД
if not exist "%DB_PATH%" (
    echo X Ошибка: База данных не найдена по пути: %DB_PATH%
    exit /b 1
)

REM Проверяем существование SQL скрипта
if not exist "%MIGRATION_SQL%" (
    echo X Ошибка: SQL миграция не найдена: %MIGRATION_SQL%
    exit /b 1
)

REM Проверяем доступность sqlite3
where sqlite3 >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo X Ошибка: sqlite3 не установлен или не в PATH
    echo Скачайте с https://www.sqlite.org/download.html
    exit /b 1
)

echo V База данных найдена: %DB_PATH%
echo V SQL миграция найдена: %MIGRATION_SQL%
echo.

REM Показываем текущее количество записей
for /f %%i in ('sqlite3 "%DB_PATH%" "SELECT COUNT(*) FROM p902_ozon_finance_realization;"') do set CURRENT_COUNT=%%i
echo Текущее количество записей в p902: %CURRENT_COUNT%
echo.

REM Создаем backup
set BACKUP_PATH=%DB_PATH%.backup_%date:~-4%%date:~3,2%%date:~0,2%_%time:~0,2%%time:~3,2%%time:~6,2%
set BACKUP_PATH=%BACKUP_PATH: =0%
echo Создаем backup: %BACKUP_PATH%
copy "%DB_PATH%" "%BACKUP_PATH%" >nul

if %ERRORLEVEL% EQU 0 (
    echo V Backup создан успешно
) else (
    echo X Ошибка создания backup
    exit /b 1
)

echo.
echo Применяем миграцию...
echo.

REM Применяем миграцию
sqlite3 "%DB_PATH%" < "%MIGRATION_SQL%"

if %ERRORLEVEL% EQU 0 (
    echo.
    echo V Миграция применена успешно!

    REM Проверяем новую схему
    for /f %%i in ('sqlite3 "%DB_PATH%" "SELECT COUNT(*) FROM p902_ozon_finance_realization;"') do set NEW_COUNT=%%i
    echo Количество записей после миграции: %NEW_COUNT%

    if "%CURRENT_COUNT%"=="%NEW_COUNT%" (
        echo V Все данные сохранены
    ) else (
        echo ! Внимание: количество записей изменилось!
    )

    echo.
    echo V Готово! Теперь можно запускать backend.
    echo Backup сохранен: %BACKUP_PATH%
) else (
    echo.
    echo X Ошибка при применении миграции
    echo Восстанавливаем из backup...
    copy "%BACKUP_PATH%" "%DB_PATH%" >nul
    echo V База данных восстановлена из backup
    exit /b 1
)

echo.
pause
