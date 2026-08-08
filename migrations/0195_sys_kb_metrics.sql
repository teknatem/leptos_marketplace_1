-- Наблюдаемая статистика по статьям базы знаний.
--
-- Источник истины по СОДЕРЖАНИЮ статьи — markdown-файл в [llm].knowledge_base_path
-- (правится в Obsidian). Здесь только НАБЛЮДЕНИЯ: обращения, чтения, цитирования,
-- замечания. Счётчики нельзя держать в frontmatter: каждое обращение LLM
-- переписывало бы файл, конфликтуя с внешним редактором.
--
-- Ключ article_key = frontmatter `uid` (если есть), иначе doc_id (имя файла).
-- doc_id и title дублируются снимком, чтобы осиротевшая после переименования
-- строка была диагностируемой, а не загадочной.

CREATE TABLE IF NOT EXISTS sys_kb_article_metrics (
    article_key       TEXT PRIMARY KEY,
    doc_id            TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    -- «поиск счёл релевантным» — инкремент только по ВОЗВРАЩЁННЫМ статьям
    search_hits       INTEGER NOT NULL DEFAULT 0,
    -- «модель реально потратила токены» — get_knowledge
    read_hits         INTEGER NOT NULL DEFAULT 0,
    -- «попало в ответ, который увидел человек» — ссылка kb://article/<id>
    cited_hits        INTEGER NOT NULL DEFAULT 0,
    issue_count       INTEGER NOT NULL DEFAULT 0,
    open_issue_count  INTEGER NOT NULL DEFAULT 0,
    token_cost_last   INTEGER NOT NULL DEFAULT 0,
    last_search_at    TEXT,
    last_read_at      TEXT,
    last_issue_at     TEXT,
    created_at        TEXT NOT NULL,
    updated_at        TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sys_kb_article_metrics_doc_id
    ON sys_kb_article_metrics (doc_id);
CREATE INDEX IF NOT EXISTS idx_sys_kb_article_metrics_reads
    ON sys_kb_article_metrics (read_hits DESC);
CREATE INDEX IF NOT EXISTS idx_sys_kb_article_metrics_issues
    ON sys_kb_article_metrics (open_issue_count DESC)
    WHERE open_issue_count > 0;

-- Замечания к статьям: append-only события со своим жизненным циклом.
-- Отдельная таблица, потому что счётчик не может нести текст и цитату.
-- open_issue_count денормализован в sys_kb_article_metrics, чтобы ранжирование
-- не делало join.
CREATE TABLE IF NOT EXISTS sys_kb_article_issue (
    id           TEXT PRIMARY KEY,
    article_key  TEXT NOT NULL,
    doc_id       TEXT NOT NULL,
    -- inaccuracy | outdated | contradiction | gap | unclear
    issue_kind   TEXT NOT NULL DEFAULT 'inaccuracy',
    -- low | normal | high
    severity     TEXT NOT NULL DEFAULT 'normal',
    body         TEXT NOT NULL,
    quote        TEXT NOT NULL DEFAULT '',
    chat_id      TEXT,
    agent_id     TEXT,
    -- agent | user
    reported_by  TEXT NOT NULL DEFAULT 'agent',
    -- поднятый тикет a031_kb_edit, если замечание эскалировали
    kb_edit_id   TEXT,
    -- open | accepted | rejected | fixed
    status       TEXT NOT NULL DEFAULT 'open',
    created_at   TEXT NOT NULL,
    resolved_at  TEXT
);

CREATE INDEX IF NOT EXISTS idx_sys_kb_article_issue_article
    ON sys_kb_article_issue (article_key, status);
CREATE INDEX IF NOT EXISTS idx_sys_kb_article_issue_created
    ON sys_kb_article_issue (created_at);
