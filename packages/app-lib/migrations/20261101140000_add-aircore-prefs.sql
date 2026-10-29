-- AirCore 自定义偏好（强调色、内存开关、下载源等）
CREATE TABLE IF NOT EXISTS aircore_prefs (
    key TEXT NOT NULL PRIMARY KEY,
    value TEXT NOT NULL
);
