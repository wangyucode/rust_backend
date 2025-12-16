-- 创建apps表
CREATE TABLE IF NOT EXISTS apps (
    appid TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    image TEXT NOT NULL,
    description TEXT NOT NULL
);

-- 插入初始测试数据到apps表
INSERT OR IGNORE INTO
    apps (
        appid,
        name,
        image,
        description
    )
VALUES (
        'app1',
        '应用1',
        'https://example.com/image1.jpg',
        '这是应用1的简介'
    ),
    (
        'app2',
        '应用2',
        'https://example.com/image2.jpg',
        '这是应用2的简介'
    ),
    (
        'app3',
        '应用3',
        'https://example.com/image3.jpg',
        '这是应用3的简介'
    );