use futures::{AsyncReadExt, AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
};

use yulong_network::{
    identity::{Peer, Me}, 
    transport::{Transport},
};

use std::{
    collections::HashMap,
    net::{
        SocketAddr, IpAddr, Ipv4Addr,
    },
    sync::mpsc::{Sender},
};

use log::{warn, info};

use crate::route::Route;

pub struct BDN<T: Transport> {

    local_identity: Me,
    listener: <T as Transport>::Listener,
    
    address_book: BidirctHashmap<Peer, SocketAddr>,
    w_stream: HashMap<Peer, <T as Transport>::Stream>,
    msg_sender: Option<Sender<Vec<u8>>>,
}

impl<T: Transport> BDN<T> {
    
    pub async fn new(port: u16) -> Self {
        Self {
            local_identity: Me::new(),

            listener: 
                T::listen(
                    &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
                ).await.ok().unwrap(),
            
            address_book: BidirctHashmap::<Peer, SocketAddr>::new(),
            
            w_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
            msg_sender: None,
        }
    }

    // accept incoming connections and spawn tasks to serve them
    pub async fn run(&mut self) {

        while match T::accept(&mut self.listener).await {
            
            Ok(istream) => {
                // a new incoming connection
                
                let peer = Peer::from_public_key(&istream.remote_pk);
                
                // If it is a new peer, add it to the address_book with a temp
                // peer id derived from provided public key.
                if !self.address_book.contains_value(&istream.remote_addr) {
                    self.address_book.insert(
                        peer.clone(),
                        istream.remote_addr.clone());
                }

                if let Some(sender) = self.msg_sender.clone() {
                    async_std::task::spawn(
                        async move {
                                Self::handle_ingress(istream.stream, sender).await;
                        });
                }
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

    pub async fn send_to(&mut self, dst: Peer, msg: &[u8]) {
        if let Some(wstream) = self.w_stream.get_mut(&dst) {
            // use existing connection to dst
            wstream.write_all(msg).await.unwrap();
        }
        else {
            // connect and send
        }
    }

    pub async fn broadcast(&self, src: Peer, msg: &[u8]) {
        
        
        
        unimplemented!()
    }

    pub async fn handle_ingress(mut s: <T as Transport>::Stream, sender: Sender<Vec<u8>>) {
        let mut buf = [0; 2048];

        match s.read(&mut buf).await {
            Ok(len) => {
                info!("BDN::handle_ingress: read {} bytes", &len);
                // collect a full message and then invoke callback
                match sender.send(buf[0..len].to_vec()) {
                    Ok(_) => {}
                    Err(error) => {
                        warn!("BDN::handle_ingress: {}", error);
                    }
                } 
            }
            Err(error) => {
                warn!("BDN::handle_ingress: {}", error);
            }
        }
    }

}


#[cfg(test)]
mod test {
    use super::BDN;
    use yulong_tcp::TcpContext;
    use async_std::{self};
    use yulong::log;

    struct User {

    }

    impl User {
        
        fn process(&mut self, buf: &[u8]) {
            println!("{:?}", buf);
        }

    }

    #[async_std::test]
    async fn new_bdn() {

        log::setup_logger().unwrap();
        
        let mut bdn = BDN::<TcpContext>::new(9001).await;

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        // bdn.run().await;
    }

}