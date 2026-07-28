-- Дедуп YM платёжного отчёта (p907): снять pending-строки «Будет … по графику выплат»,
-- у которых уже есть проведённый двойник (тот же order_id / transaction_type / shop_sku /
-- |transaction_sum|; статус двойника НЕ «Будет …» и НЕ «Справочно …»).
--
-- Прогноз выплаты («Будет переведён по графику выплат») и факт («Переведён по графику выплат»)
-- — одни и те же деньги на разных стадиях. Сейчас обе строки попадают и в колонку суммирования
-- документа a013, и в GL (customer_revenue по transaction_source «Платёж покупателя») → задвой.
-- Реальный transaction_id от YM пустой (NULL), поэтому сопоставляем по бизнес-полям; дату НЕ
-- учитываем — прогноз может быть датирован иначе, чем фактическая выплата.
--
-- Разовая чистка уже накопленных дублей (в т.ч. legacy-хвосты формата ключа SYNTH_ до ymid_).
-- Появление новых предотвращает гард импорта u503 → service::purge_superseded_pending_payouts
-- (тот же предикат). Снимаем строку целиком: GL (sys_general_ledger), p914, p915 и саму p907.
--
-- Порядок важен: сначала зависимые проводки (пока строки p907 ещё на месте), затем сама p907;
-- каждый DELETE пересчитывает один и тот же набор «мёртвых» id одинаковым подзапросом.
-- Репост не нужен: отчёты/обороты читают sys_general_ledger вживую, p914 — зеркало (чистится),
-- перечисления (Дт51/Кт7609) не затронуты — у снимаемых pending-строк bank_sum = NULL.

DELETE FROM sys_general_ledger
WHERE registrator_type = 'p907_ym_payment_report'
  AND registrator_ref IN (
    SELECT p.id FROM p907_ym_payment_report p
    WHERE p.payment_status LIKE 'Будет %'
      AND EXISTS (
        SELECT 1 FROM p907_ym_payment_report s
        WHERE s.order_id = p.order_id
          AND s.transaction_type = p.transaction_type
          AND IFNULL(s.shop_sku, '') = IFNULL(p.shop_sku, '')
          AND CAST(ROUND(ABS(s.transaction_sum) * 100) AS INTEGER)
            = CAST(ROUND(ABS(p.transaction_sum) * 100) AS INTEGER)
          AND s.payment_status NOT LIKE 'Будет %'
          AND s.payment_status NOT LIKE 'Справочно%'
          AND s.id <> p.id
      )
  );

DELETE FROM p914_mp_finance_turnovers
WHERE registrator_ref IN (
    SELECT p.id FROM p907_ym_payment_report p
    WHERE p.payment_status LIKE 'Будет %'
      AND EXISTS (
        SELECT 1 FROM p907_ym_payment_report s
        WHERE s.order_id = p.order_id
          AND s.transaction_type = p.transaction_type
          AND IFNULL(s.shop_sku, '') = IFNULL(p.shop_sku, '')
          AND CAST(ROUND(ABS(s.transaction_sum) * 100) AS INTEGER)
            = CAST(ROUND(ABS(p.transaction_sum) * 100) AS INTEGER)
          AND s.payment_status NOT LIKE 'Будет %'
          AND s.payment_status NOT LIKE 'Справочно%'
          AND s.id <> p.id
      )
  );

DELETE FROM p915_mp_order_events
WHERE registrator_ref IN (
    SELECT p.id FROM p907_ym_payment_report p
    WHERE p.payment_status LIKE 'Будет %'
      AND EXISTS (
        SELECT 1 FROM p907_ym_payment_report s
        WHERE s.order_id = p.order_id
          AND s.transaction_type = p.transaction_type
          AND IFNULL(s.shop_sku, '') = IFNULL(p.shop_sku, '')
          AND CAST(ROUND(ABS(s.transaction_sum) * 100) AS INTEGER)
            = CAST(ROUND(ABS(p.transaction_sum) * 100) AS INTEGER)
          AND s.payment_status NOT LIKE 'Будет %'
          AND s.payment_status NOT LIKE 'Справочно%'
          AND s.id <> p.id
      )
  );

DELETE FROM p907_ym_payment_report
WHERE id IN (
    SELECT p.id FROM p907_ym_payment_report p
    WHERE p.payment_status LIKE 'Будет %'
      AND EXISTS (
        SELECT 1 FROM p907_ym_payment_report s
        WHERE s.order_id = p.order_id
          AND s.transaction_type = p.transaction_type
          AND IFNULL(s.shop_sku, '') = IFNULL(p.shop_sku, '')
          AND CAST(ROUND(ABS(s.transaction_sum) * 100) AS INTEGER)
            = CAST(ROUND(ABS(p.transaction_sum) * 100) AS INTEGER)
          AND s.payment_status NOT LIKE 'Будет %'
          AND s.payment_status NOT LIKE 'Справочно%'
          AND s.id <> p.id
      )
  );
