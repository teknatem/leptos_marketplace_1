-- Normalize user-facing descriptions for indicators used in dashboard `test`.

UPDATE a024_bi_indicator
SET comment = 'Фактическая выручка маркетплейса за выбранный период и кабинеты.'
WHERE code = 'REVENUE';

UPDATE a024_bi_indicator
SET comment = 'Выручка по цене реализации из оперативных GL-оборотов MP за выбранный период.'
WHERE code = 'IND-MP-REV-PRICE';

UPDATE a024_bi_indicator
SET comment = 'Отношение фактической выручки к выручке по цене реализации, в процентах.'
WHERE code = 'IND-REV-TO-PRICE-PCT';

UPDATE a024_bi_indicator
SET comment = 'Итог по счету 7609 только по основным оборотам, с учетом периода и кабинета.'
WHERE code = 'IND-GL-7609-MAIN-BALANCE';

UPDATE a024_bi_indicator
SET comment = 'Отношение итога счета 7609 по основным оборотам к выручке по цене реализации, в процентах.'
WHERE code = 'IND-GL-7609-TO-PRICE-PCT';

UPDATE a024_bi_indicator
SET comment = 'Сумма возвратов маркетплейса по фактическим GL-оборотам за выбранный период.'
WHERE code = 'IND-MP-RETURNS';

UPDATE a024_bi_indicator
SET comment = 'Количество возвратов по обороту customer_revenue_pl_storno за выбранный период.'
WHERE code = 'IND-MP-RETURNS-COUNT';

UPDATE a024_bi_indicator
SET comment = 'Доля возвратов в реализации: сумма customer_revenue_pl_storno к customer_revenue_pl, в процентах.'
WHERE code = 'IND-MP-RETURNS-TO-REV-PCT';

UPDATE a024_bi_indicator
SET comment = 'Себестоимость реализованных товаров по оперативным GL-оборотам.'
WHERE code = 'COST';

UPDATE a024_bi_indicator
SET comment = 'Комиссия маркетплейса по оперативным GL-оборотам за выбранный период.'
WHERE code = 'IND-MP-COMMISSION';

UPDATE a024_bi_indicator
SET comment = 'Сумма соинвеста маркетплейса по оперативным GL-оборотам.'
WHERE code = 'IND-MP-COINVEST';

UPDATE a024_bi_indicator
SET comment = 'Расходы на рекламу Wildberries за выбранный период.'
WHERE code = 'IND-WB-ADS-SPEND';

UPDATE a024_bi_indicator
SET comment = 'Количество заказов за выбранный период по выбранным кабинетам.'
WHERE code = 'IND-ORDERS';

UPDATE a024_bi_indicator
SET comment = 'Прибыль дилера за выбранный период.'
WHERE code = 'IND-PROFIT-D';

UPDATE a024_bi_indicator
SET comment = 'Средний чек без возвратов за выбранный период.'
WHERE code = 'IND-AVG-CHECK';

UPDATE a024_bi_indicator
SET comment = 'Эквайринг маркетплейса по фактическим GL-оборотам.'
WHERE code = 'IND-GL-MP-ACQ-FACT';

UPDATE a024_bi_indicator
SET comment = 'Штрафы маркетплейса по фактическим GL-оборотам.'
WHERE code = 'IND-GL-MP-PENALTY-FACT';

UPDATE a024_bi_indicator
SET comment = 'Логистика маркетплейса по фактическим GL-оборотам.'
WHERE code = 'IND-GL-MP-LOGISTICS-FACT';
