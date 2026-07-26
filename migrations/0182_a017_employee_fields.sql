-- a017_llm_agent становится «виртуальным сотрудником»: персона поверх технического
-- подключения a038. Добавляем поля сотрудника (неразрушающе — старые технические
-- колонки остаются вестигиальными). connection_id сидим = id: у a017 и a038 совпадают
-- UUID (миграция 0165 копировала a017→a038 с сохранением id), поэтому у каждого
-- сотрудника стартовое подключение — его «близнец» a038 с тем же id.

ALTER TABLE a017_llm_agent ADD COLUMN connection_id TEXT;
ALTER TABLE a017_llm_agent ADD COLUMN avatar TEXT;
ALTER TABLE a017_llm_agent ADD COLUMN email TEXT;
ALTER TABLE a017_llm_agent ADD COLUMN schedule_cron TEXT;
ALTER TABLE a017_llm_agent ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

-- Сидинг привязки к подключению: self-match к a038-близнецу с тем же UUID,
-- но только если такой a038 реально существует (иначе оставляем NULL).
UPDATE a017_llm_agent
SET connection_id = id
WHERE connection_id IS NULL
  AND EXISTS (SELECT 1 FROM a038_llm_connection c WHERE c.id = a017_llm_agent.id);

-- Если близнеца нет — привязываем к основному подключению (is_primary), если оно есть.
UPDATE a017_llm_agent
SET connection_id = (
    SELECT c.id FROM a038_llm_connection c
    WHERE c.is_primary = 1 AND c.is_deleted = 0
    LIMIT 1
)
WHERE connection_id IS NULL;

CREATE INDEX IF NOT EXISTS idx_a017_connection_id ON a017_llm_agent(connection_id);
CREATE INDEX IF NOT EXISTS idx_a017_agent_type ON a017_llm_agent(agent_type);
CREATE INDEX IF NOT EXISTS idx_a017_is_active ON a017_llm_agent(is_active);
