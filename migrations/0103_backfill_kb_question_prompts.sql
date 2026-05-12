-- Make existing question tickets actionable when an earlier prompt created them
-- without explicit questions.

UPDATE a031_kb_edit
SET
    agent_summary = agent_summary || char(10) || char(10) ||
        '## Вопросы к пользователю' || char(10) ||
        '1. В каких бизнес-процессах компании используется тема этого тикета?' || char(10) ||
        '2. Кто в компании отвечает за этот процесс или принимает решения по нему?' || char(10) ||
        '3. Какие данные, документы или системы являются источником информации?' || char(10) ||
        '4. Какие правила, исключения или спорные ситуации важно знать бизнес-аналитику?' || char(10) ||
        '5. Какие отчёты, показатели или управленческие решения зависят от этого знания?',
    updated_at = datetime('now'),
    version = version + 1
WHERE edit_type = 'question'
  AND is_deleted = 0
  AND agent_summary NOT LIKE '%Вопросы к пользователю%';
