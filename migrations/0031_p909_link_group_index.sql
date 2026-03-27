-- Speeds up link-status refresh during p909 repost/posting.
-- Hot query: WHERE connection_mp_ref = ? AND line_event_key = ? AND turnover_code = ?

CREATE INDEX IF NOT EXISTS idx_p909_link_group
    ON p909_mp_order_line_turnovers (connection_mp_ref, line_event_key, turnover_code);
