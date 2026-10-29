-- 去商业化：移除“跨设备同步”（依赖 Modrinth 账号）相关的设置列
ALTER TABLE settings DROP COLUMN sync_theme_across_devices;
ALTER TABLE settings DROP COLUMN sync_behavior_across_devices;
ALTER TABLE settings DROP COLUMN sync_features_across_devices;
