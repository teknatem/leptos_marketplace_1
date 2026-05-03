-- Add nm_count field to store the number of positions (nm_settings) in WB advert campaign
ALTER TABLE a030_wb_advert_campaign ADD COLUMN nm_count INTEGER NOT NULL DEFAULT 0;
