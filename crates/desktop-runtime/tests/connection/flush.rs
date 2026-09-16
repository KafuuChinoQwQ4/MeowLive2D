use futures_util::StreamExt;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, protocol::Role},
};

#[tokio::test(start_paused = true)]
async fn automatic_pong_flush_cannot_wait_forever_on_a_blocked_writer() {
    // A one-byte output buffer cannot fit the masked Pong. Peer writes a
    // standards-compliant server Ping and deliberately never consumes output.
    let (client, mut peer) = tokio::io::duplex(1);
    let mut socket = WebSocketStream::from_raw_socket(client, Role::Client, None).await;
    let ((), received) = tokio::join!(
        async {
            peer.write_all(&[0x89, 0]).await.unwrap();
        },
        socket.next()
    );
    assert!(matches!(received, Some(Ok(Message::Ping(_)))));
    let start = tokio::time::Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        super::flush(&mut socket, Duration::from_secs(1), "control"),
    )
    .await
    .expect("flush must have its own write timeout");
    assert!(
        result
            .unwrap_err()
            .contains("control heartbeat write timed out")
    );
    assert_eq!(start.elapsed(), Duration::from_secs(1));
}
