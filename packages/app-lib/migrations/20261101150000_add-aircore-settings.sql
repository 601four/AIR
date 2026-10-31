-- AirCore: 把原本存在 aircore_prefs 里的用户可见设置合并进 settings 表
-- 迁移完成后删除 aircore_prefs

ALTER TABLE settings ADD COLUMN accent_color TEXT NOT NULL DEFAULT '#ff5050';
ALTER TABLE settings ADD COLUMN auto_memory BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN prelaunch_optimize BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN download_source TEXT NOT NULL DEFAULT 'official';
ALTER TABLE settings ADD COLUMN loader_mode TEXT NOT NULL DEFAULT 'cn';
ALTER TABLE settings ADD COLUMN loader_prefix TEXT;
ALTER TABLE settings ADD COLUMN modrinth_mirror BOOLEAN NOT NULL DEFAULT 0;
ALTER TABLE settings ADD COLUMN sidebar_news BOOLEAN NOT NULL DEFAULT 1;

DROP TABLE IF EXISTS aircore_prefs;
