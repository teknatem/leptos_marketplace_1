-- Специализация «Тестировщик» — обкатка пайплайна на локальной модели (Ollama).
--
-- Роль отдельная не ради новых умений, а ради изоляции: урезать матрицу навыков
-- существующей специализации значило бы задеть облачных сотрудников, которые на ней
-- работают. У локальной модели контекст в разы меньше, поэтому набор навыков (а с ним
-- и объём схем инструментов в промпте) держим минимальным.
--
-- Строки формально не обязательны (дефолт для неизвестной пары = denied), но без
-- хотя бы одного immediate-навыка тестировщик получит только core-инструменты.

INSERT OR IGNORE INTO sys_llm_skill_access (specialization, skill_id, access_level) VALUES
    ('tester', 'support',            'immediate'),
    ('tester', 'quality-monitoring', 'immediate'),
    ('tester', 'data-analytics',     'extended');

-- Публикация артефактов тестировщику не нужна: build_chart/build_table/plugin_upsert
-- останутся недоступны (см. MUTATING_ARTIFACT_TOOLS в skill_policy.rs).
INSERT OR IGNORE INTO sys_llm_specialization_capability (specialization, capability, is_allowed) VALUES
    ('tester', 'artifact_publish', 0);
