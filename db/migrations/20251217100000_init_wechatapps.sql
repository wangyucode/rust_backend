-- 创建wechat_apps表
CREATE TABLE IF NOT EXISTS wechat_apps (
    id TEXT NOT NULL PRIMARY KEY,
    appid TEXT NOT NULL,
    name TEXT NOT NULL,
    img TEXT NOT NULL,
    note TEXT NOT NULL
);

-- 插入初始数据
INSERT INTO
    wechat_apps (id, appid, name, img, note)
VALUES (
        'roll',
        'wxa6e870e9d665b10b',
        '3D滚蛋吧',
        'https://wycode.cn/apps/roll.jpg',
        '一起来玩滚蛋吧，看谁滚的远...😂'
    ),
    (
        'clipboard',
        'wx1977172112eb7b61',
        '微翼云空间',
        'https://wycode.cn/apps/clipboard.jpg',
        '无需登录，跨平台，跨设备记录文字、网址'
    ),
    (
        'oni',
        'wx16945b3b638b065c',
        'oni产物计算器',
        'https://wycode.cn/apps/oni.jpg',
        '提供《缺氧》产物平衡计算功能'
    );