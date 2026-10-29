-- 提高默认下载并发：10 -> 32（仅在仍是旧默认值时调整）
UPDATE settings SET max_concurrent_downloads = 32 WHERE max_concurrent_downloads = 10;
