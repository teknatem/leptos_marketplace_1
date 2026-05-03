-- HTTP / traffic metrics per task run (WB API and future external calls)
ALTER TABLE sys_task_runs ADD COLUMN http_request_count INTEGER;
ALTER TABLE sys_task_runs ADD COLUMN http_bytes_sent INTEGER;
ALTER TABLE sys_task_runs ADD COLUMN http_bytes_received INTEGER;
