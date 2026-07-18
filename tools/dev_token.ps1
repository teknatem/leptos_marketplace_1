# Получение dev-токена (Bearer) для вызова API из терминала/скриптов.
#
# Логинится в запущенный backend (http://localhost:3000 по умолчанию) и печатает
# access_token — одной строкой, без кавычек. Удобно для curl / Invoke-RestMethod:
#
#   $t = powershell -File tools/dev_token.ps1
#   curl -H "Authorization: Bearer $t" http://localhost:3000/api/a018-llm-chat/with-stats
#
# Креды: параметры -Username/-Password, иначе переменные окружения
# APP_DEV_LOGIN / APP_DEV_PASSWORD, иначе дефолт admin/admin (создаётся
# автоматически при пустой таблице пользователей — см. system/initialization.rs).
# Токен и пароль не логируются.

param(
    [string]$Username = $(if ($env:APP_DEV_LOGIN) { $env:APP_DEV_LOGIN } else { "admin" }),
    [string]$Password = $(if ($env:APP_DEV_PASSWORD) { $env:APP_DEV_PASSWORD } else { "admin" }),
    [string]$BaseUrl = "http://localhost:3000"
)

$ErrorActionPreference = "Stop"

$body = @{ username = $Username; password = $Password } | ConvertTo-Json -Compress
try {
    $resp = Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/system/auth/login" `
        -ContentType "application/json" -Body $body
} catch {
    Write-Error "Login failed for user '$Username' at $BaseUrl : $($_.Exception.Message)"
    exit 1
}

$resp.access_token
