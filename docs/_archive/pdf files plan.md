# a018 LLM-чат: файлы как S3-артефакты + распознавание PDF + read_artifact

## Контекст

В чате a018 две проблемы, которые пользователь решил объединить вокруг механизма **артефактов**:

1. **Файл не виден в переписке.** `.md` (2.7 КБ) прикрепляется, но «не появляется». Причина структурная, не фильтр: вложения рендерятся только как эфемерные пред-отправочные чипы (`uploaded_files`, [view.rs:720](crates/frontend/src/domain/a018_llm_chat/ui/details/view.rs#L720)), сигнал очищается при отправке, пузыри сообщений не читают `msg.attachments`, а репозиторий грузит сообщения с пустым `attachments` ([repository.rs:239](crates/backend/src/domain/a018_llm_chat/repository.rs#L239)).
2. **PDF не распознаётся.** Контент вложений читается через `tokio::fs::read_to_string` ([service.rs:1487](crates/backend/src/domain/a018_llm_chat/service.rs#L1487)) — только UTF-8; PDF молча пропускается. Рабочий экстрактор `pdf_extract` уже есть в [pdf_report_extractor.rs:73](crates/backend/src/domain/a027_wb_documents/pdf_report_extractor.rs#L73) (приватный, внутри a027).

**Утверждённое направление (решения пользователя):**
- **Каждый загруженный файл** (pdf, md, csv, txt…) становится **Document-артефактом** a019 — первоклассным, видимым в чате, ссылающимся объектом. Это через один механизм решает и задачу 1 (видимость), и задачу 2 (PDF).
- **Файлы хранятся в S3**, в БД — только ссылка (файлов будет много, база растёт быстро). Инлайн-хранение в a019 — запасной вариант (S3 выключен).
- **Модель читает артефакт по требованию**: в контекст авто-попадает короткая справка (имя, тип, размер, id), модель тянет содержимое новым тулом `read_artifact(id[, диапазон])`. Как в Claude Code: файл — объект, который читают осмысленно, не раздувая каждый ход.
- Лимит текста одного вложения поднять с 64 КБ до ~200 КБ.

## Что переиспользуем (готовая инфраструктура)

- **S3**: `crate::system::s3::service::{upload, download, delete}` (кладёт в S3 + пишет строку-ссылку в `sys_files_s3`, возвращает `S3FileDto{id,…}`), типы `UploadedFile`/`S3FileDto`/`S3FileCategory` ([contracts/src/system/s3.rs](crates/contracts/src/system/s3.rs)), конвенция ключей `category/YYYY/MM/<uuid>/<file>` ([system/s3/service.rs:49](crates/backend/src/system/s3/service.rs#L49)). Требует `[s3].enabled=true` + bucket/creds ([config.rs:177](crates/backend/src/shared/config.rs#L177)).
- **Артефакты**: `crate::domain::a019_llm_artifact::service::create(LlmArtifactDto)` ([service.rs:27](crates/backend/src/domain/a019_llm_artifact/service.rs#L27)), `repository::{find_by_id,list_by_chat_id}`, `ArtifactType` ([contracts a019 aggregate.rs:38](crates/contracts/src/domain/a019_llm_artifact/aggregate.rs#L38)), карточка `ArtifactCard` ([artifact_card.rs](crates/frontend/src/domain/a018_llm_chat/ui/details/artifact_card.rs)). Паттерн «не-SQL артефакт»: `sql_query=""`, полезная нагрузка в `query_params` JSON (как drilldown/plugin) — **миграция a019 не нужна**.
- **PDF**: `pdf_extract` (уже в [Cargo.toml:42](crates/backend/Cargo.toml#L42)); логику из [pdf_report_extractor.rs:73](crates/backend/src/domain/a027_wb_documents/pdf_report_extractor.rs#L73) (обяз. `catch_unwind`).
- **Send-контекст**: `send_message` ([service.rs:454](crates/backend/src/domain/a018_llm_chat/service.rs#L454)); `chat.agent_id` доступен (line 471) — им создаём артефакт.

---

## Фаза 1 — S3-хранение загрузок + распознавание + видимость (задачи 1 и 2)

**Общий PDF-экстрактор.** Новый `crates/backend/src/shared/pdf/mod.rs`: `pub fn extract_text_from_bytes(&[u8]) -> anyhow::Result<String>` (перенести паттерн `catch_unwind`+`pdf_extract::extract_text_from_mem` из a027; a027 переключить на него — убрать дубль).

**Категория S3.** Добавить `S3FileCategory::ChatUploads` (`"chat_uploads"`) в [contracts/src/system/s3.rs](crates/contracts/src/system/s3.rs) (`as_str`, `label_ru`, `all`, `From<&str>`). Ключи → `chat_uploads/YYYY/MM/<uuid>/<file>`.

**Загрузка → S3 + извлечение текста.** В `upload_attachment` ([service.rs:1407](crates/backend/src/domain/a018_llm_chat/service.rs#L1407)):
- Если `[s3].enabled`: сырой файл → `system::s3::service::upload(ChatUploads, UploadedFile{filename,content_type,bytes}, user)` → `s3_file_id`. Извлечь текст (PDF → `shared::pdf` в `tokio::task::spawn_blocking`; текстовые → UTF-8) → загрузить `.txt` в S3 → `s3_text_id` (для текстовых файлов `s3_text_id=None`, читаем сам файл). Извлечение неуспешно → не фатально: файл всё равно сохранён и виден.
- Иначе (S3 off) — текущий локальный диск (`uploads/chat_attachments/…`), текст inline в новую колонку `extracted_text` (fallback).

**Миграция вложения.** `migrations/NNNN_a018_attachment_s3.sql`: добавить в `a018_llm_chat_attachment` колонки `s3_file_id TEXT`, `s3_text_id TEXT`, `artifact_id TEXT`, `extracted_text TEXT`, `char_count INTEGER`, `page_count INTEGER` (все nullable). Отразить в SeaORM `attachment::Model` ([repository.rs:91](crates/backend/src/domain/a018_llm_chat/repository.rs#L91)) и в `LlmChatAttachment` ([aggregate.rs:205](crates/contracts/src/domain/a018_llm_chat/aggregate.rs#L205)).

**Видимость (задача 1).** Бэкенд: при загрузке сообщений заполнять `msg.attachments` (`repository::find_attachments_by_message_id`, батчем) вместо `Vec::new()` ([repository.rs:239](crates/backend/src/domain/a018_llm_chat/repository.rs#L239)) в обработчике `GET /:id/messages`. Фронтенд: под пузырём ([view.rs:72](crates/frontend/src/domain/a018_llm_chat/ui/details/view.rs#L72)) рендерить чипы из `msg.attachments` (стиль из [view.rs:732](crates/frontend/src/domain/a018_llm_chat/ui/details/view.rs#L732)); показывать вложения оптимистично при отправке ([model.rs:283](crates/frontend/src/domain/a018_llm_chat/ui/details/model.rs#L283)); `.pdf` в `accept` ([view.rs:764](crates/frontend/src/domain/a018_llm_chat/ui/details/view.rs#L764)); тело ошибки аплоада вместо голого 500 ([a018_llm_chat.rs:441](crates/backend/src/api/handlers/a018_llm_chat.rs#L441)); `DefaultBodyLimit::max(25MB)` на роут upload ([routes.rs:866](crates/backend/src/api/routes.rs#L866)).

---

## Фаза 2 — Document-артефакты + read_artifact + короткая справка в контексте

**Тип артефакта.** Добавить `ArtifactType::Document` (`"document"`) в [contracts a019 aggregate.rs:38](crates/contracts/src/domain/a019_llm_artifact/aggregate.rs#L38) (`from_str`/`as_str`).

**Создание артефакта при отправке.** В `send_message`, при привязке вложений к пользовательскому сообщению: для каждого вложения `a019::service::create(LlmArtifactDto{ chat_id, agent_id: chat.agent_id, artifact_type: Document, code: gen, description: filename, comment: "PDF · N стр · размер", sql_query: "", query_params: JSON{ s3_file_id, s3_text_id, filename, mime, size, char_count, page_count } })`; полученный `artifact_id` записать в `attachment.artifact_id`; на пользовательское сообщение проставить `artifact_id` первого файла (для многих файлов карточки рендерим из `msg.attachments`). agent_id — тот же способ резолва, что у существующих артефактов (chat.agent_id).

**Карточка Document.** Ветка `Document` в `ArtifactCard` ([artifact_card.rs](crates/frontend/src/domain/a018_llm_chat/ui/details/artifact_card.rs)): заголовок = имя файла, мета (тип/страницы/размер), кнопки «Открыть/Скачать» (скачивание через S3 download route). Иначе Document упадёт в SQL-ветку. Ветка в details-view a019 ([a019.../ui/details/view.rs](crates/frontend/src/domain/a019_llm_artifact/ui/details/view.rs)).

**Тул `read_artifact`.** Новый LLM-тул в `crates/backend/src/shared/llm` (определение + диспетч в [tool_executor.rs](crates/backend/src/shared/llm/tool_executor.rs), регистрация в бандле/скилле). Сигнатура: `read_artifact(artifact_id, offset?, limit?)` → загрузить артефакт (`a019::repository::find_by_id`), из `query_params` взять `s3_text_id`/`s3_file_id`, скачать текст (`system::s3::service::download`; если S3 off — из `attachment.extracted_text`/локального файла), вернуть постранично/по срезу с пометкой усечения. Описание тула — с триггером «вызывай, когда нужно содержимое прикреплённого документа/отчёта». Заодно `list_artifacts(chat_id)` (по `list_by_chat_id`), если модель потеряла id.

**Контекст: короткая справка вместо полного текста.** Заменить текущую авто-инъекцию полного текста ([service.rs:489-515](crates/backend/src/domain/a018_llm_chat/service.rs#L489) и `append_attachments_text` [service.rs:1498](crates/backend/src/domain/a018_llm_chat/service.rs#L1498)) на компактную строку на каждый Document-артефакт: `📎 [artifact_id] filename (PDF, N стр, ~M симв) — read_artifact для чтения` + превью первых ~1 КБ (чтобы крошечные файлы часто отвечались без вызова). Лимит `MAX_ATTACHMENT_FILE_BYTES` 64 КБ → 200 КБ ([service.rs:1495](crates/backend/src/domain/a018_llm_chat/service.rs#L1495)) — применяется к тексту, отдаваемому `read_artifact`.

---

## Ключевые файлы

| Файл | Изменение |
|---|---|
| `crates/backend/src/shared/pdf/mod.rs` | **новый** — `extract_text_from_bytes` |
| `crates/backend/src/domain/a027_wb_documents/pdf_report_extractor.rs` | переключить на `shared::pdf` |
| `crates/contracts/src/system/s3.rs` | категория `ChatUploads` |
| `crates/contracts/src/domain/a019_llm_artifact/aggregate.rs` | `ArtifactType::Document` |
| `migrations/NNNN_a018_attachment_s3.sql` | колонки s3/artifact/extracted у вложения |
| `crates/backend/src/domain/a018_llm_chat/service.rs` | S3-загрузка+извлечение; создание Document-артефакта при отправке; короткая справка; лимит 200 КБ |
| `crates/backend/src/domain/a018_llm_chat/repository.rs` | новые колонки в entity; заполнять `msg.attachments` |
| `crates/contracts/src/domain/a018_llm_chat/aggregate.rs` | новые поля `LlmChatAttachment` |
| `crates/backend/src/shared/llm/tool_executor.rs` (+ реестр тулов) | тул `read_artifact` / `list_artifacts` |
| `crates/backend/src/api/handlers/a018_llm_chat.rs`, `api/routes.rs` | тело ошибки аплоада; `DefaultBodyLimit` |
| `crates/frontend/.../artifact_card.rs`, `a019.../ui/details/view.rs` | ветка Document |
| `crates/frontend/.../ui/details/{view.rs,model.rs}` | чипы на пузыре, `.pdf`, оптимистичный показ |

---

## Верификация

1. `cargo check -p backend && cargo check -p contracts && cargo check -p frontend --target wasm32-unknown-unknown`.
2. `cargo test -p backend router_builds` (после правок роутов + `read_artifact` + DefaultBodyLimit).
3. Одноразовый `cargo run -p backend` до «Routes configured» (миграция применится; конфликты роутов — только при старте; занятый `backend.exe` → `tools/restart_backend.ps1`). Убедиться, что `[s3].enabled=true` с bucket/ключами (иначе тестируем локальный fallback-путь).
4. `trunk serve`, чат `…3c6d14e7…`:
   - Прикрепить `yandex_market_report_2026-06_amounts.md` → чип на пузыре виден до и **после** отправки и после F5; в S3 появился объект `chat_uploads/…`, в `sys_files_s3` строка; создан Document-артефакт (карточка «Открыть/Скачать»).
   - Прикрепить `542956908_agent_rep_2026-06-30_64526156.pdf` (1.4 МБ) → аплоад проходит; спросить модель по содержимому → модель вызывает `read_artifact` (виден в tool-trace) и отвечает числами из PDF; текст извлечён (в S3 `.txt`-объект непустой).
   - Ошибка аплоада → `vm.error` показывает причину.
5. a027 WB-weekly-report импорт не сломан после переезда экстрактора в `shared/`.
6. Fallback: `[s3].enabled=false` → загрузка идёт на локальный диск, `read_artifact` читает `extracted_text`/локальный файл.
