use yulong_quic::QuicContext;
use yulong_network::transport::Transport;
use futures::{AsyncReadExt};

#[tokio::main]
async fn main() {
    if let Ok(mut listener) = QuicContext::listen(&"0.0.0.0:4433".parse().unwrap()).await {
        if let Ok(mut stream) = QuicContext::accept(&mut listener).await {
            let mut buf = [0u8; 1024];
            while let Ok(len) = stream.stream.read(&mut buf).await {
                println!("recv {:?}", &buf[..len]);
            }
        }
    }
}