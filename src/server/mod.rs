pub mod dto;
pub mod error;
pub mod handlers;
pub mod routes;

use std::{net::SocketAddr, sync::OnceLock};

use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngCore;
use tokio::{
    net::TcpListener,
    sync::{Mutex, oneshot},
};

use crate::core;
use crate::server::handlers::AppState;

pub struct ServerInfo {
    pub addr: SocketAddr,
    pub token: String,
}

struct ServerHandle {
    shutdown_tx: oneshot::Sender<()>,
    addr: SocketAddr,
    token: String,
}

static SERVER: OnceLock<Mutex<Option<ServerHandle>>> = OnceLock::new();

fn server_cell() -> &'static Mutex<Option<ServerHandle>> {
    SERVER.get_or_init(|| Mutex::new(None))
}

fn gen_token() -> String {
    let mut b = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

/// port=0 => 随机端口
pub async fn start(port: u16) -> Result<ServerInfo> {
    core::init_logger();

    let mut guard = server_cell().lock().await;
    if let Some(h) = guard.as_ref() {
        // 已经启动过就直接返回（幂等）
        return Ok(ServerInfo {
            addr: h.addr,
            token: h.token.clone(),
        });
    }

    let token = gen_token();
    let state = AppState {
        token: token.clone(),
    };

    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    let addr = listener.local_addr()?;

    let app = routes::router(state);

    let (tx, rx) = oneshot::channel::<()>();

    core::runtime().spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = rx.await;
            })
            .await;
    });

    *guard = Some(ServerHandle {
        shutdown_tx: tx,
        addr,
        token: token.clone(),
    });

    Ok(ServerInfo { addr, token })
}
pub async fn stop() -> Result<()> {
    let mut guard = server_cell().lock().await;
    if let Some(h) = guard.take() {
        let _ = h.shutdown_tx.send(());
    }
    Ok(())
}
