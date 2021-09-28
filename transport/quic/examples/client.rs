use yulong_quic::QuicContext;
use yulong_network::transport::Transport;
use futures::{AsyncWriteExt};

#[tokio::main]
async fn main() {
    if let Ok(mut stream) = QuicContext::connect(&"0.0.0.0:4433".parse().unwrap()).await {
        let mut buf = [0u8; 100];
        for i in 0..100 {
            buf[i] = i as u8;
            let _len = stream.write(&buf[..i]).await.unwrap();
        }
    }
}