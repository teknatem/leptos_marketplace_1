# Date Period Filtering

## Problem

In this project many SQLite columns named `date` are stored as ISO/RFC3339 text with time, for example:

- `2026-03-31`
- `2026-03-31T12:45:00+03:00`

At the same time UI period filters usually send bounds in `YYYY-MM-DD`.

Direct string comparison is unsafe for period end:

```sql
WHERE t.date >= ? AND t.date <= ?
```

With `date_to = '2026-03-31'` this excludes rows like `2026-03-31T12:45:00+03:00`.

## Rule

When the business filter is a calendar day period, compare by the date part only:

```sql
WHERE substr(t.date, 1, 10) >= ?
  AND substr(t.date, 1, 10) <= ?
```

This keeps the period end inclusive for both plain dates and datetime strings.

## Where It Applies

- DataViews and dashboard SQL over `p904_sales_data`
- generic drilldown queries
- projection list filters by period
- universal dashboard date filters

## Review Checklist

- If DB stores `TEXT` datetime and API sends `YYYY-MM-DD`, do not compare raw strings.
- For grouping by day, use `DATE(column)` or `substr(column, 1, 10)`.
- For period filters, normalize both bounds to day granularity before `>=`, `<=`, or `BETWEEN`.
