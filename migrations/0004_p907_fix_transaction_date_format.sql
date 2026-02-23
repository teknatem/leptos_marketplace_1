-- 0004 Fix p907_ym_payment_report transaction_date format
-- Converts dates from Russian format DD.MM.YYYY HH:MM to ISO format YYYY-MM-DD HH:MM
-- Root cause: CSV from Yandex Market uses DD.MM.YYYY HH:MM, but date filters use ISO string comparison
-- Condition: detect old format by checking that position 3 and 6 are dots (DD.MM.YYYY ...)

UPDATE p907_ym_payment_report
SET transaction_date =
    SUBSTR(transaction_date, 7, 4) || '-' ||
    SUBSTR(transaction_date, 4, 2) || '-' ||
    SUBSTR(transaction_date, 1, 2) ||
    SUBSTR(transaction_date, 11)
WHERE transaction_date IS NOT NULL
  AND SUBSTR(transaction_date, 3, 1) = '.'
  AND SUBSTR(transaction_date, 6, 1) = '.';
