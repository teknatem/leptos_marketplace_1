# Задать вопрос внутреннему LLM-чату (a018) по API и дождаться ответа.
#
# Использование:
#   $t = powershell -File tools/dev_token.ps1 -Username claude_dev -Password ...
#   powershell -File tools/ask_internal_chat.ps1 -Token $t -Question "Выручка за июнь?"
#
# Параметры:
#   -Token     Bearer-токен (см. tools/dev_token.ps1)
#   -Question  текст вопроса
#   -ChatId    id существующего чата; если не задан — создаётся новый
#   -AgentId   id подключения LLM (a038); если не задан — берётся первое из списка
#   -TimeoutSec бюджет ожидания ответа (по умолчанию 360)
#
# Печатает JSON: { chat_id, job_seconds, answer, tokens_used, model, tool_trace }

param(
    [Parameter(Mandatory = $true)][string]$Token,
    [string]$Question = "",
    # Файл с вопросом в UTF-8 — надёжный способ передать кириллицу (аргументы
    # командной строки у powershell.exe конвертируются через OEM-кодировку и бьются).
    [string]$QuestionFile = "",
    [string]$ChatId = "",
    [string]$AgentId = "",
    [string]$BaseUrl = "http://localhost:3000",
    [int]$TimeoutSec = 360,
    # Куда сохранить JSON-результат (UTF-8). Пусто — только stdout.
    [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

if ($QuestionFile) {
    $Question = (Get-Content -Path $QuestionFile -Raw -Encoding UTF8).Trim()
}
if (-not $Question) { Write-Error "Provide -Question or -QuestionFile"; exit 1 }

$headers = @{ Authorization = "Bearer $Token" }

# PS 5.1: строковое тело кодируется Latin-1, а ответ без charset декодируется
# Latin-1 — поэтому шлём байты UTF-8 и декодируем ответ из сырых байтов сами.
function Invoke-JsonApi([string]$Method, [string]$Uri, [string]$Json = $null) {
    $params = @{ Method = $Method; Uri = $Uri; Headers = $headers; UseBasicParsing = $true }
    if ($Json) {
        $params.Body = [System.Text.Encoding]::UTF8.GetBytes($Json)
        $params.ContentType = "application/json; charset=utf-8"
    }
    $resp = Invoke-WebRequest @params
    $text = [System.Text.Encoding]::UTF8.GetString($resp.RawContentStream.ToArray())
    if ($text) { $text | ConvertFrom-Json }
}

# 1. Чат: существующий или новый
if (-not $ChatId) {
    if (-not $AgentId) {
        $conns = Invoke-JsonApi "Get" "$BaseUrl/api/a038-llm-connection"
        if (-not $conns -or $conns.Count -eq 0) { Write-Error "No LLM connections (a038)"; exit 1 }
        $primary = $conns | Where-Object { $_.is_primary } | Select-Object -First 1
        if (-not $primary) { $primary = $conns[0] }
        $AgentId = $primary.base.id
        if (-not $AgentId) { $AgentId = $primary.id }
    }
    $createBody = @{ description = "CC-compare: $($Question.Substring(0, [Math]::Min(60, $Question.Length)))"; agent_id = $AgentId } | ConvertTo-Json -Compress
    $created = Invoke-JsonApi "Post" "$BaseUrl/api/a018-llm-chat" $createBody
    $ChatId = $created.id
}

# 2. Отправить сообщение (202 + job_id)
$sendBody = @{ content = $Question; attachment_ids = @(); request_id = "cc-" + [guid]::NewGuid().ToString() } | ConvertTo-Json -Compress
$job = Invoke-JsonApi "Post" "$BaseUrl/api/a018-llm-chat/$ChatId/messages" $sendBody
$jobId = $job.job_id

# 3. Поллинг до done/error
$sw = [System.Diagnostics.Stopwatch]::StartNew()
$answer = $null
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    Start-Sleep -Seconds 2
    $st = Invoke-JsonApi "Get" "$BaseUrl/api/a018-llm-chat/jobs/$jobId"
    if ($st.status -eq "done") { $answer = $st.message; break }
    if ($st.status -eq "error") { Write-Error "LLM job error: $($st.error)"; exit 1 }
}
$sw.Stop()
if (-not $answer) { Write-Error "Timeout after $TimeoutSec s (job $jobId, chat $ChatId)"; exit 1 }

$result = [pscustomobject]@{
    chat_id     = $ChatId
    job_seconds = [Math]::Round($sw.Elapsed.TotalSeconds, 1)
    answer      = $answer.content
    tokens_used = $answer.tokens_used
    model       = $answer.model_name
    tool_trace  = $answer.tool_trace
} | ConvertTo-Json -Depth 4

if ($OutFile) {
    [System.IO.File]::WriteAllText($OutFile, $result, [System.Text.Encoding]::UTF8)
}
$result
