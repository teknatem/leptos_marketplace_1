-- User rating for a plugin (1..5; NULL = not rated).
ALTER TABLE plugin ADD COLUMN rating INTEGER;
