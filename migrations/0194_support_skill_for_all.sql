-- Навык поддержки доступен всем специализациям.
--
-- Пожаловаться на работу программы должен уметь любой чат, а не отдельная персона:
-- иначе пользователю приходится заранее решать, «вопрос по данным» у него или
-- «вопрос по системе», и выбирать вход. Тему определяет роутер интентов по самой
-- формулировке, а инструменты навыка безопасны: свои тикеты и чтение базы знаний.

INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level) VALUES
    ('business_analyst', 'support', 'immediate'),
    ('sales_analyst',    'support', 'immediate'),
    ('marketer',         'support', 'immediate'),
    ('financier',        'support', 'immediate'),
    ('system_admin',     'support', 'immediate'),
    ('kb_admin',         'support', 'immediate');
