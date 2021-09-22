use tokio;
use yulong_quic::QuicContext;
use yulong_network::transport::Transport;

#[tokio::main]
async fn main() {
    if let Ok(mut stream) = QuicContext::connect("127.0.0.1:4433".parse().unwrap()).await {
        let mut buf = [0u8; 100];
        for i in 0..100 {
            buf[i] = i as u8;
            let _len = stream.send_stream.write(&buf[..i]).await.unwrap();
        }
    }
}