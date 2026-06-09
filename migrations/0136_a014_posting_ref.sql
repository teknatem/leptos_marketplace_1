-- a014_ozon_transactions: добавляем ссылку на отправление (posting), к которому
-- привязана транзакция, и тип регистратора (A010 = FBS, A011 = FBO).
-- Поля уже присутствуют в SeaORM-сущности и доменной логике (posting.rs,
-- service_enrichment.rs), но соответствующая миграция была пропущена,
-- из-за чего SELECT по таблице падал с "no such column: posting_ref".

ALTER TABLE a014_ozon_transactions ADD COLUMN posting_ref TEXT;
ALTER TABLE a014_ozon_transactions ADD COLUMN posting_ref_type TEXT;
