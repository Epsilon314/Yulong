use futures::{AsyncWriteExt};
use yulong_network::{transport::{Transport}};
use yulong_tcp::TcpContext;
use yulong_quic::QuicContext;
use std::{error::Error, net::SocketAddr, str::FromStr};


#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    client::<QuicContext>().await;
    Ok(())
}


async fn client<T: Transport>() {
    
    let mut stream = T::connect(
        &SocketAddr::from_str("127.0.0.1:9001").ok().unwrap()
    ).await.ok().unwrap();

    let buf :[u8; 5] = [1,2,3,4,5];
    for _ in 0..200 {
        stream.write(&buf).await.unwrap();
        // stream.flush().await.unwrap();
    }
    stream.close().await.unwrap();
}
