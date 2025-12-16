-- 创建apps表
CREATE TABLE IF NOT EXISTS apps (
    id TEXT NOT NULL PRIMARY KEY,
    appid TEXT NOT NULL,
    name TEXT NOT NULL,
    image TEXT NOT NULL,
    description TEXT NOT NULL
);