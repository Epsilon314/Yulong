use std::{error::Error, net::SocketAddr, str::FromStr};
use futures::{AsyncReadExt};
use yulong_network::{transport::{IngressStream, Transport}};
use yulong_tcp::TcpContext;
use yulong_quic::QuicContext;

use async_std::task;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    server::<QuicContext>().await;
    Ok(())
}

async fn server<T: Transport>() {

    let mut listener: T::Listener = 
        T::listen(
            &SocketAddr::from_str("0.0.0.0:9001").ok().unwrap()
        ).await.ok().unwrap();

    loop {
        match T::accept(&mut listener).await {
            Ok(stream) => {
                task::spawn(
                    async move {connection::<T>(stream).await}
                );
            }
            Err(_) => {println!("Connection error.");}
        };
    }
}

async fn connection<'a, T: Transport> (stream: IngressStream<<T as Transport>::Stream>) {
    
    let mut accepted_stream = stream.stream;
    let remote_addr = stream.remote_addr;
    println!("Connected by {:?}", remote_addr);

    let mut buf = [0u8; 2048];

    while match accepted_stream.read(&mut buf).await {
        Ok(0_usize) => {
            false
        }
        Ok(size) => {
            println!("Received: {:?}", String::from_utf8(buf[0..size].to_vec()).ok().unwrap());
            true
        }
        Err(_) => {
            false
        }
    } {}
}
