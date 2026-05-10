-- Начальное заполнение полей comment и explanation для всех индикаторов a024_bi_indicator.

-- ---------------------------------------------------------------------------
-- GL-first: отдельные обороты (dv004)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Реализация по ценам прайса из GL-оборота customer_revenue_pl.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот customer_revenue_pl, слой oper. Отражает начисленную стоимость по ценам номенклатуры до применения скидок и вычета возвратов. Мигрирован из dv003 в 2026-04. Используется как знаменатель в ratio-индикаторах IND-REV-TO-PRICE-PCT и IND-GL-7609-TO-PRICE-PCT.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-REV-PRICE';

UPDATE a024_bi_indicator SET
    comment = 'Чистая реализация МП: выручка минус возвраты покупателей из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Формула: customer_revenue + customer_return на слое fact (возвраты в GL хранятся со знаком минус, поэтому сложение даёт нетто-выручку). Мигрирован из dv003 в 2026-04 (миграция 0071) для унификации с GL-first подходом.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-REV';

UPDATE a024_bi_indicator SET
    comment = 'Суммарное ко-инвестирование WB по операционным проводкам главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот wb_coinvestment, слой oper. Ко-инвестирование — участие WB в финансировании акций и скидок продавца. Сумма выражена в рублях и является расходом для продавца. Мигрирован из dv003 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COINVEST';

UPDATE a024_bi_indicator SET
    comment = 'Расходы на эквайринг МП по операционным проводкам главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот mp_acquiring, слой oper. Слой oper фиксирует эквайринг в момент операции, в отличие от IND-GL-MP-ACQ-FACT (слой fact из финотчёта МП). Мигрирован из dv003 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-ACQUIRING';

UPDATE a024_bi_indicator SET
    comment = 'Себестоимость проданных товаров по операционным проводкам главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот item_cost, слой oper. Себестоимость единицы товара списывается при каждой продаже. Значение отрицательно (расход), поэтому при анализе прибыльности прибавляется к выручке. Мигрирован из dv003 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COST';

UPDATE a024_bi_indicator SET
    comment = 'Комиссия маркетплейса по операционным проводкам главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот mp_commission, слой oper. Комиссия удерживается с каждой продажи в зависимости от категории товара и условий кабинета. Мигрирован из dv003 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-COMMISSION';

UPDATE a024_bi_indicator SET
    comment = 'Сумма возвратов покупателей по факту из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот customer_return, слой fact. Отражает фактически зачтённые суммы возвратов из финансового отчёта МП. Для подсчёта количества возвратов (а не суммы) используется IND-MP-RETURNS-COUNT. Мигрирован из dv003 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS';

UPDATE a024_bi_indicator SET
    comment = 'Расходы на рекламу Wildberries из GL-оборота advertising_allocated.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Оборот advertising_allocated, слой oper. Отражает рекламные затраты, разнесённые по номенклатуре из отчётов WB по рекламным кампаниям. Drilldown привязан к детализации p911_wb_advert_by_items. Мигрирован из dv002 в 2026-04.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-WB-ADS-SPEND';

-- ---------------------------------------------------------------------------
-- GL-first: fact-индикаторы (dv004)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Сумма эквайринга маркетплейса по факту из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Источник: sys_general_ledger, оборот mp_acquiring, слой fact. Слой fact соответствует строкам из официального финансового отчёта маркетплейса. В отличие от IND-MP-ACQUIRING (слой oper), этот индикатор отражает окончательно зачтённые суммы.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-MP-ACQ-FACT';

UPDATE a024_bi_indicator SET
    comment = 'Сумма штрафов и удержаний маркетплейса по факту из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Источник: sys_general_ledger, оборот mp_penalty, слой fact. Штрафы — удержания за нарушения SLA, ненадлежащую упаковку и другие условия договора с маркетплейсом.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-MP-PENALTY-FACT';

UPDATE a024_bi_indicator SET
    comment = 'Суммарные логистические расходы маркетплейса по факту из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Формула по нескольким оборотам: mp_ppvz_reward + mp_ppvz_reward_nm + mp_rebill_logistic_cost + mp_rebill_logistic_cost_nm, слой fact. Охватывает вознаграждение WB за логистику и начисления за хранение и доставку из финансового отчёта.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-MP-LOGISTICS-FACT';

-- ---------------------------------------------------------------------------
-- Счёт 7609 (dv005)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Итоговое сальдо счёта 7609 по основным оборотам за период.',
    explanation = 'DataView: dv005_gl_account_view_total, metric=balance. Параметры: account=7609, section=main. Счёт 7609 используется для учёта взаиморасчётов с маркетплейсами. Секция main включает основные операционные обороты, исключая информационные строки секции info. Сальдо рассчитывается как разница дебетовых и кредитовых оборотов за период.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-7609-MAIN-BALANCE';

-- ---------------------------------------------------------------------------
-- Ratio-индикаторы (dv006)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Доля фактической выручки к прайсовой цене реализации, в процентах.',
    explanation = 'DataView: dv006_indicator_ratio_percent, metric=ratio_percent. Формула: REVENUE / IND-MP-REV-PRICE * 100. Числитель REVENUE — выручка из p904_sales_data (dv001). Знаменатель IND-MP-REV-PRICE — GL-оборот customer_revenue_pl/oper из главной книги. Показывает, какую долю от максимально возможной выручки по прайсу фактически получил продавец после всех скидок и возвратов.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-REV-TO-PRICE-PCT';

UPDATE a024_bi_indicator SET
    comment = 'Доля итога счёта 7609 к реализации по прайсу, в процентах.',
    explanation = 'DataView: dv006_indicator_ratio_percent, metric=ratio_percent. Формула: IND-GL-7609-MAIN-BALANCE / IND-MP-REV-PRICE * 100. Числитель — сальдо счёта 7609 по основным оборотам, знаменатель — реализация по ценам прайса. Используется для анализа, какая доля прайсовой выручки осела на счёте взаиморасчётов с МП.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-7609-TO-PRICE-PCT';

UPDATE a024_bi_indicator SET
    comment = 'Дополнение к 100%: доля потерь выручки относительно прайса.',
    explanation = 'DataView: dv006_indicator_ratio_percent, metric=ratio_percent_complement. Формула: 100 - (REVENUE / IND-MP-REV-PRICE * 100). Показывает, какой процент прайсовой выручки был потерян из-за скидок, возвратов и прочих удержаний. Является зеркальным дополнением к IND-REV-TO-PRICE-PCT.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-REV-TO-PRICE-COMPLEMENT-PCT';

UPDATE a024_bi_indicator SET
    comment = 'Дополнение к 100% для доли счёта 7609 к прайсовой реализации.',
    explanation = 'DataView: dv006_indicator_ratio_percent, metric=ratio_percent_complement. Формула: 100 - (IND-GL-7609-MAIN-BALANCE / IND-MP-REV-PRICE * 100). Зеркальный показатель к IND-GL-7609-TO-PRICE-PCT: отражает долю прайсовой выручки, не прошедшей через счёт 7609.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-GL-7609-TO-PRICE-COMPLEMENT-PCT';

-- ---------------------------------------------------------------------------
-- Индикаторы из dv001 (p904_sales_data)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Суммарная выручка по кабинетам Wildberries за выбранный период.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: SUM(customer_in + customer_out) по всем строкам за период. Фильтр по кабинетам МП задаётся параметром connection_ids. Сравнительный период вычисляется автоматически как предыдущий месяц или задаётся явно через period2_from / period2_to. DataView: dv001_revenue, metric=revenue.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-REVENUE-WB';

UPDATE a024_bi_indicator SET
    comment = 'Количество уникальных заказов по кабинетам WB за выбранный период.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: COUNT(DISTINCT registrator_ref). Каждый уникальный registrator_ref соответствует отдельному документу-регистратору. DataView: dv001_revenue, metric=order_count.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-ORDERS';

UPDATE a024_bi_indicator SET
    comment = 'Дилерская прибыль: выручка плюс себестоимость по кабинетам WB.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: SUM(customer_in + customer_out + cost). Себестоимость в p904_sales_data хранится со знаком минус, поэтому сложение с выручкой даёт разницу выручка минус себестоимость. DataView: dv001_revenue, metric=profit_d.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-PROFIT-D';

UPDATE a024_bi_indicator SET
    comment = 'Средний чек: отношение реализации по прайсу к количеству заказов.',
    explanation = 'DataView: dv006_indicator_ratio_percent, metric=ratio. Числитель: IND-MP-REV-PRICE (GL-оборот customer_revenue_pl, слой oper). Знаменатель: IND-ORDERS (COUNT DISTINCT registrator_ref из p904_sales_data). Результат выражен в рублях. При нулевом знаменателе значение не вычисляется.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-AVG-CHECK';

-- ---------------------------------------------------------------------------
-- Возвраты (dv007)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Количество операций возврата покупателей за период.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=entry_count. Источник: sys_general_ledger, оборот customer_revenue_pl_storno, слой oper. Каждая строка GL с этим оборотом соответствует одному возврату. Используется для мониторинга количества возвратных операций независимо от их суммы — в отличие от IND-MP-RETURNS, который отражает сумму.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS-COUNT';

UPDATE a024_bi_indicator SET
    comment = 'Доля суммы возвратов к реализации по ценам прайса, в процентах.',
    explanation = 'DataView: dv007_gl_turnover_ratio_percent, metric=ratio_percent. Числитель: оборот -customer_revenue_pl_storno (стоимость возвратов с инверсией знака) на слое oper. Знаменатель: оборот customer_revenue_pl на том же слое. Показывает, какой процент стоимости реализации был возвращён покупателями. Ориентир: значение выше 10-15% требует анализа причин.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'IND-MP-RETURNS-TO-REV-PCT';

-- ---------------------------------------------------------------------------
-- Generic draft-индикаторы (REVENUE, COMM, EXP, PROFIT, RET, COST)
-- ---------------------------------------------------------------------------

UPDATE a024_bi_indicator SET
    comment = 'Суммарная выручка по всем кабинетам МП за период.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: SUM(customer_in + customer_out). Базовый агрегат выручки, используется как числитель в ratio-индикаторах (IND-REV-TO-PRICE-PCT). DataView: dv001_revenue, metric=revenue.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'REVENUE';

UPDATE a024_bi_indicator SET
    comment = 'Суммарная комиссия маркетплейса по данным p904_sales_data.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: SUM(commission_out). Draft-индикатор на dv001_revenue. Для точного учёта по GL рекомендуется использовать IND-MP-COMMISSION.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'COMM';

UPDATE a024_bi_indicator SET
    comment = 'Суммарные расходы (эквайринг + штрафы + логистика) по данным p904_sales_data.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: SUM(acquiring_out + penalty_out + logistics_out). Draft-индикатор на dv001_revenue. Для раздельного учёта компонентов используйте GL-first индикаторы: IND-GL-MP-ACQ-FACT, IND-GL-MP-PENALTY-FACT, IND-GL-MP-LOGISTICS-FACT.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'EXP';

UPDATE a024_bi_indicator SET
    comment = 'Прибыль продавца по данным p904_sales_data.',
    explanation = 'Источник данных: таблица p904_sales_data. Формула: -SUM(seller_out). seller_out — итоговое перечисление продавцу со знаком минус, поэтому применяется инверсия. Draft-индикатор на dv001_revenue, metric=profit.',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'PROFIT';

UPDATE a024_bi_indicator SET
    comment = 'Сумма возвратов покупателей по данным p904_sales_data.',
    explanation = 'Источник данных: таблица p904_sales_data. Draft-индикатор на dv001_revenue. Для GL-first учёта используйте IND-MP-RETURNS (слой fact).',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'RET';

UPDATE a024_bi_indicator SET
    comment = 'Себестоимость проданных товаров: item_cost + item_cost_storno из главной книги.',
    explanation = 'DataView: dv004_general_ledger_turnovers, metric=amount. Формула: item_cost + item_cost_storno, слой oper. item_cost — начисленная себестоимость при продаже; item_cost_storno — её сторнирование при возврате. Сумма двух оборотов даёт чистую себестоимость с учётом возвратов. Обновлён до GL-first в 2026-04 (миграция 0052).',
    updated_at = datetime('now'), version = COALESCE(version, 0) + 1
WHERE code = 'COST';
