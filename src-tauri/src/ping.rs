//! Замер задержки до серверов.
//!
//! ponytail: меряем время TCP-подключения, а не HTTP-запрос через прокси, как
//! делает URL-тест в NekoBox — для второго нужно тащить в бандл xray-core.
//! Задержку это показывает честную, работоспособность ключа — нет.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

const TIMEOUT: Duration = Duration::from_secs(3);
const CONCURRENCY: usize = 32;

/// Миллисекунды до установки TCP-соединения. None — не ответил за таймаут.
/// DNS резолвится до замера, чтобы не приплюсовывать его к задержке.
pub async fn ping(host: &str, port: u16) -> Option<u32> {
    let address = tokio::time::timeout(TIMEOUT, lookup_host((host, port)))
        .await
        .ok()?
        .ok()?
        .next()?;

    let started = Instant::now();
    tokio::time::timeout(TIMEOUT, TcpStream::connect(address))
        .await
        .ok()?
        .ok()?;
    Some(started.elapsed().as_millis() as u32)
}

/// Проверяет весь список параллельно, сохраняя порядок.
pub async fn ping_all(targets: Vec<(String, u16)>) -> Vec<Option<u32>> {
    let mut results = vec![None; targets.len()];
    let limit = Arc::new(Semaphore::new(CONCURRENCY));
    let mut tasks = JoinSet::new();

    for (index, (host, port)) in targets.into_iter().enumerate() {
        let limit = limit.clone();
        tasks.spawn(async move {
            let _permit = limit.acquire_owned().await;
            (index, ping(&host, port).await)
        });
    }

    while let Some(Ok((index, latency))) = tasks.join_next().await {
        results[index] = latency;
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn measures_open_port_and_gives_up_on_closed_one() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // порт, который точно никто не слушает
        let closed = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let closed_port = closed.local_addr().unwrap().port();
        drop(closed);

        let results = ping_all(vec![
            ("127.0.0.1".into(), port),
            ("127.0.0.1".into(), closed_port),
            ("несуществующий.invalid".into(), 443),
        ])
        .await;

        assert!(results[0].is_some(), "открытый порт должен отвечать");
        assert!(results[1].is_none(), "закрытый порт не должен отвечать");
        assert!(results[2].is_none(), "неразрешимый хост не должен отвечать");
    }
}
