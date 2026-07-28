# Номенклатура 1С и товарные измерения

Основной каталог товаров — `a004_nomenclature`, импортированный из 1С:УТ11. Категорию
маркетплейса из `a007.category_name` не используй вместо классификации 1С.

## Шесть основных измерений `a004`

| Поле | Смысл |
|---|---|
| `dim1_category` | категория |
| `dim2_line` | линия |
| `dim3_model` | модель |
| `dim4_format` | формат |
| `dim5_sink` | мойка |
| `dim6_size` | размер |

Пустое измерение обозначай как «Не заполнено», но не смешивай его с реальным значением.
Папки (`is_folder = 1`) не являются продаваемыми SKU.

## Маппинг маркетплейса на 1С

`a007_marketplace_product` — каталог товаров маркетплейсов. Его поле `nomenclature_ref`
ссылается на `a004_nomenclature.id`.

Для WB канонический ключ товара:

1. `(connection_mp_ref, marketplace_sku = CAST(nm_id AS TEXT))`;
2. фолбэк — уникальный `(connection_mp_ref, article)`;
3. `u505_match_nomenclature` устанавливает `a007.nomenclature_ref` по уникальному артикулу;
   отсутствие или неоднозначность совпадения оставляет связь пустой.

`p916` уже несёт `marketplace_product_ref` и `nomenclature_ref`, но у старых/маркетинговых
строк ссылки могут отсутствовать. Для устойчивого join используй `nm_id` как мост:

```sql
LEFT JOIN a007_marketplace_product mp
  ON mp.connection_mp_ref = f.connection_mp_ref
 AND mp.marketplace_sku = CAST(f.nm_id AS TEXT)
 AND mp.is_deleted = 0
LEFT JOIN a004_nomenclature n
  ON n.id = COALESCE(NULLIF(f.nomenclature_ref,''), mp.nomenclature_ref)
 AND n.is_deleted = 0
 AND n.is_folder = 0
```

Для группировки выбирай одно или несколько измерений `n.dim1_category` … `n.dim6_size`.
Не группируй один и тот же товар одновременно по `a007.category_name` и `a004.dim1_category`
как будто это одна классификация.

## Диагностика покрытия маппинга

```sql
WITH products AS (
  SELECT DISTINCT connection_mp_ref, nm_id, nomenclature_ref
  FROM p916_mp_sales_funnel_turnovers
  WHERE cohort_date BETWEEN ? AND ?
    AND connection_mp_ref = ?
)
SELECT COUNT(*) AS sku_total,
       SUM(CASE WHEN COALESCE(NULLIF(p.nomenclature_ref,''),mp.nomenclature_ref) IS NOT NULL
                THEN 1 ELSE 0 END) AS sku_mapped,
       SUM(CASE WHEN COALESCE(NULLIF(p.nomenclature_ref,''),mp.nomenclature_ref) IS NULL
                THEN 1 ELSE 0 END) AS sku_unmapped
FROM products p
LEFT JOIN a007_marketplace_product mp
  ON mp.connection_mp_ref = p.connection_mp_ref
 AND mp.marketplace_sku = CAST(p.nm_id AS TEXT)
 AND mp.is_deleted = 0
```

Если `sku_unmapped > 0`, не включай такие SKU в сравнение по измерениям без отдельной группы
«Не сопоставлено» и предупреждения о покрытии. После изменения маппинга через `u505` документы
нужно перепровести, чтобы денормализованный `nomenclature_ref` в проекциях отразил актуальную
связь.
