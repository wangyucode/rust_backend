use anyhow::Result;
use sqlx::SqlitePool;

use crate::dao::app::get_all_apps;
use crate::util::email;

/// 启动前业务逻辑
pub async fn after_startup(pool: &SqlitePool) -> Result<()> {
    // 查询wechat_apps数据
    let apps = get_all_apps(pool).await?;

    println!("\n📊 迁移后应用数据：");
    for app in apps {
        println!(
            "应用ID：{}，名称：{}，图片：{}，描述：{}",
            app.appid, app.name, app.img, app.note
        );
    }

    println!("\n🚀 业务服务启动成功");

    // 发送启动通知邮件
    let start_notification = format!(
        "Rust后端服务已成功启动！\n\n时间：{}\n版本：{}",
        chrono::Local::now().to_string(),
        env!("CARGO_PKG_VERSION")
    );
    let email_config = email::EmailConfig::new(
        Some("【Rust】后端服务启动通知".to_string()),
        start_notification,
        None,
    );

    if let Err(e) = email::send_email(email_config) {
        eprintln!("发送启动通知邮件失败：{}", e);
    } else {
        println!("已发送启动通知邮件");
    }

    Ok(())
}
