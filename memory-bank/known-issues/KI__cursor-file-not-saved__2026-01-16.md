---
type: known-issue
date: 2026-01-16
severity: medium
tags: [cursor, tooling, file-sync]
---

# Known Issue: Cursor File Changes Not Saved to Disk

## Symptom

- `read_file` показывает правильное содержимое файла
- `cargo check` выдаёт ошибки "function not found"
- Ощущение что изменения не применились

## Detection

Проверить через terminal:

```powershell
type path\to\file.rs | Select-String "function_name"
```

Если вернёт пустой результат — файл не сохранён на диск.

## Root Cause

Cursor иногда держит изменения в буфере/кеше без записи на диск, особенно при использовании `search_replace` tool.

## Fix

Принудительная запись через PowerShell:

```powershell
@"
<file content here>
"@ | Out-File -FilePath "path\to\file.rs" -Encoding utf8
```

Альтернатива: использовать `write` tool с полным содержимым файла.

## Prevention

После крупных изменений проверять через terminal что файл действительно обновился.
