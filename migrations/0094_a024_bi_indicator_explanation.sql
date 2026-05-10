-- Add long-form user explanation for BI indicators.

ALTER TABLE a024_bi_indicator
    ADD COLUMN explanation TEXT;
