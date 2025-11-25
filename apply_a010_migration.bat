@echo off
echo Applying migration for a010_ozon_fbs_posting...
sqlite3 marketplace.db < migrate_a010_add_substatus.sql
if %errorlevel% equ 0 (
    echo Migration applied successfully!
) else (
    echo Migration failed with error code %errorlevel%
)
pause

