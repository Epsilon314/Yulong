use futures::{AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
};

use yulong_network::{identity::{Me, Peer, crypto::AsBytes}, transport::Transport};

use std::{collections::HashMap, net::{
        SocketAddr, IpAddr, Ipv4Addr,
    }, sync::mpsc};

use log::{warn, info};

use async_std::{io::BufReader};

use crate::{message::{self, Message}, route::Route};


type MessageWithIp = (IpAddr, Vec<u8>);

pub struct BDN<T: Transport> {

    local_identity: Me,
    address_book: BidirctHashmap<Peer, SocketAddr>,
    w_stream: HashMap<Peer, <T as Transport>::Stream>,
    msg_sender: mpsc::Sender<MessageWithIp>,
    msg_receiver: mpsc::Receiver<MessageWithIp>,
}


impl<T: Transport> BDN<T> {
    
    pub async fn new() -> Self {

        let (sender, receiver) = 
            mpsc::channel::<MessageWithIp>();

        Self {
            local_identity: Me::new(),
            
            address_book: BidirctHashmap::<Peer, SocketAddr>::new(),
            
            w_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
            msg_sender: sender,
            msg_receiver: receiver,
        }
    }

    // accept incoming connections and spawn tasks to serve them
    pub async fn run(listen_port: u16, msg_sender: mpsc::Sender<MessageWithIp>) {

        let mut listener = T::listen(
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), listen_port)
        ).await.ok().unwrap();

        while match T::accept(&mut listener).await {
            
            Ok(istream) => {
                // a new incoming connection

                let ip = istream.remote_addr.ip();
                let sender = msg_sender.clone();
                
                async_std::task::spawn(
                    async move {
                        Self::handle_ingress(istream.stream, sender, ip).await;
                });

                true
            }

            Err(error) => {
                warn!("BDN::run: {}", error);
                true
            }

        } {}
    }


    pub async fn connect(&mut self) {
        for (peer, addr) in self.address_book.iter() {
            if self.w_stream.contains_key(peer) {
                continue;
            }
            match T::connect(addr).await {
                Ok(stream) => {
                    self.w_stream.insert(
                        peer.clone(),
                        stream
                    );
                }
                Err(error) => {
                    warn!("BDN::connect: {}", error);
                }
            }
        }
    }

    pub async fn send_to(&mut self, dst: &Peer, msg: &[u8]) {

        let wrapped_msg = Message::new(msg);
        if wrapped_msg.is_err() {
            warn!("BDN::send_to: {}", wrapped_msg.unwrap_err());
            return;
        }

        let wrapped_msg = wrapped_msg.unwrap();

        // send through an existing stream 
        if let Some(wstream) = self.w_stream.get_mut(&dst) {
            // use existing connection to dst
            wstream.write_all(&wrapped_msg.into_bytes()).await.unwrap_or_else(|err| {
                warn!("BDN::send_to write error: {}", err);
            });
            return;
        }

        // no established stream, connect and send
        let addr = self.address_book.get_by_key(&dst);
        
        if addr.is_none() {
            warn!("BDN::send_to unknown dst: {:?}", dst.get_id());
            return;
        }

        let addr = addr.unwrap();
        match T::connect(addr).await {
            
            Ok(mut wstream) => {
                wstream.write_all(&wrapped_msg.into_bytes()).await
                    .unwrap_or_else( |err| {
                        warn!("BDN::send_to write error: {}", err);
                    });
                self.w_stream.insert(dst.clone(), wstream);
            }

            Err(error) => {
                warn!("BDN::send_to encounter an error when connecting {}. Error: {}", addr, error);
            }
        }
    }

    pub async fn broadcast(&self, src: Peer, msg: &[u8]) {
        unimplemented!()
    }


    pub async fn send_to_indirect(&self, dst: Peer, msg: &[u8]) {
        unimplemented!()
    }

    pub async fn handle_ingress(
        s: <T as Transport>::Stream,
        sender: mpsc::Sender<MessageWithIp>,
        remote_ip: IpAddr
    ) {

        // create a message reader with an inner buffered reader
        let mut msg_reader = message::MessageReader::<T>::new(
            BufReader::with_capacity(message::MSG_MAXLEN, s)
        );

        loop {

            // read one message at a time
            let msg= msg_reader.read_message().await;

            // encounter an ill-formed message
            if msg.is_err() {
                warn!("BDN::handle_ingress: {}", msg.unwrap_err());
                continue;
            }

            // EOF, end this processing task
            let msg = msg.unwrap();
            if msg.is_none() {
                break;
            }

            let raw_msg = msg.unwrap().get_payload();

            info!("BDN::handle_ingress: read {} bytes", raw_msg.len());
            let send_res = sender.send((remote_ip ,raw_msg));
            if send_res.is_err() {
                warn!("BDN::handle_ingress: {}", send_res.unwrap_err());
            }
        }
    }
}


#[cfg(test)]
mod test {
    use std::{net::{SocketAddrV4}, str::FromStr};

    use super::BDN;
    use yulong_tcp::TcpContext;
    use async_std::{self};
    use yulong::log;
    use yulong_network::identity::Peer;
    use std::net::SocketAddr;

    #[async_std::test]
    async fn bdn_1() {

        log::setup_logger().unwrap();
        
        let mut bdn = BDN::<TcpContext>::new().await;

        let peer = Peer::from_random();

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        bdn.address_book.insert(
            peer.clone(),
            SocketAddr::V4(SocketAddrV4::from_str("127.0.0.1:9002").unwrap())
        );

        let payload = [42_u8; 2000];
        
        let server = async_std::task::spawn(
            BDN::<TcpContext>::run(9001, bdn.msg_sender.clone())
        );

        bdn.connect().await;
        bdn.send_to(&peer, &[1,2,3]).await;
        bdn.send_to(&peer, &[1,2,3,4,5,6]).await;
        bdn.send_to(&peer, &payload).await;
        bdn.send_to(&peer, &[1,2,3]).await;

        server.await;
    }

    #[async_std::test]
    async fn bdn_2() {

        log::setup_logger().unwrap();
        
        let mut bdn = BDN::<TcpContext>::new().await;

        let peer = Peer::from_random();

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        bdn.address_book.insert(
            peer.clone(),
            SocketAddr::V4(SocketAddrV4::from_str("127.0.0.1:9001").unwrap())
        );

        let payload = [42_u8; 2000];
        
        let server = async_std::task::spawn(
            BDN::<TcpContext>::run(9002, bdn.msg_sender.clone())
        );
        
        bdn.connect().await;
        bdn.send_to(&peer, &[1,2,3]).await;
        bdn.send_to(&peer, &[1,2,3,4,5,6]).await;
        bdn.send_to(&peer, &payload).await;
        bdn.send_to(&peer, &[1,2,3]).await;
        server.await;
    }

}