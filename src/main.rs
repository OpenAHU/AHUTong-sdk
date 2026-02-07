use ahutong_rs::{core, server};
use log::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 初始化日志
    core::init_logger();

    info!("Starting AHUTong Server...");

    // 2. 启动服务 (0 means random port, but for local debug we might want fixed port, e.g. 3000)
    // Or we can let it be random and print it.
    // User requirement: "独立可执行入口（用于桌面端或调试）"
    let port = 3000;
    let info = server::start(port).await?;

    info!("Server started at http://{}", info.addr);
    info!("Token: {}", info.token);
    info!("Press Ctrl+C to stop");

    // 3. 等待退出信号
    tokio::signal::ctrl_c().await?;
    
    info!("Stopping server...");
    server::stop().await?;
    
    Ok(())
}
