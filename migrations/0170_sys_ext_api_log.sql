-- Лог входящих вызовов внешнего API (/api/ext/v1/*).
-- Строка на запрос: внешних потребителей мало (1С + Power BI), объём небольшой.
-- Пишется слоем system/ext_api_log/middleware.rs, который висит на ext-саброутере
-- снаружи check_api_key — поэтому сюда попадают и 401 (неверный ключ), и 503.
CREATE TABLE IF NOT EXISTS sys_ext_api_log (
    id           TEXT PRIMARY KEY,
    ts           TEXT    NOT NULL,   -- UTC ISO8601
    method       TEXT    NOT NULL,
    route        TEXT    NOT NULL,   -- MatchedPath: '/api/ext/v1/wb-stocks'
    path         TEXT    NOT NULL,   -- фактический путь
    query        TEXT,               -- сырая query-строка: чем контролировать корректность
    status       INTEGER NOT NULL,
    duration_ms  INTEGER NOT NULL,
    bytes_out    INTEGER NOT NULL,
    client_ip    TEXT,
    user_agent   TEXT,
    client_id    TEXT                -- NULL; задел под многоключевость внешнего API
);

CREATE INDEX IF NOT EXISTS idx_sys_ext_api_log_ts     ON sys_ext_api_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_sys_ext_api_log_route  ON sys_ext_api_log(route);
CREATE INDEX IF NOT EXISTS idx_sys_ext_api_log_status ON sys_ext_api_log(status);
