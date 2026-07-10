-- Local bookkeeping of the last successful S3 publish for a plugin.
-- Source of truth for "latest available version" remains plugins/catalog.json in S3;
-- these columns are only used to show "previously published vN at ..." in the publish dialog.
ALTER TABLE plugin ADD COLUMN s3_published_version INTEGER;
ALTER TABLE plugin ADD COLUMN s3_published_at TEXT;
ALTER TABLE plugin ADD COLUMN s3_sha256 TEXT;
