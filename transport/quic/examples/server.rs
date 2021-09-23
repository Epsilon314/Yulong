use tokio;
use yulong_quic::QuicContext;
use yulong_network::transport::Transport;

#[tokio::main]
async fn main() {
    if let Ok(mut listener) = QuicContext::listen("127.0.0.1:4433".parse().unwrap()).await {
        if let Ok(mut stream) = QuicContext::accept(&mut listener).await {
            let mut buf = [0u8; 1024];
            while let Ok(len) = stream.stream.recv_stream.read(&mut buf).await {
                match len {
                    Some(len) => println!("recv {:?}", &buf[..len]),
                    None => continue,
                }
            }
        }
    }
}