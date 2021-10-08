use futures::{AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
};

use yulong_network::{identity::{Me, Peer, crypto::AsBytes}, transport::Transport};

use std::{collections::HashMap, net::{
        SocketAddr, IpAddr, Ipv4Addr,
    }, sync::mpsc::Sender,};

use log::{warn, info};

use async_std::io::BufReader;

use crate::{message, route::Route};

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

        // send through an existing stream 
        if let Some(wstream) = self.w_stream.get_mut(&dst) {
            // use existing connection to dst
            wstream.write_all(msg).await.unwrap_or_else(|err| {
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
                wstream.write_all(msg).await.unwrap_or_else(|err| {
                    warn!("BDN::send_to write error: {}", err);
                });
                self.w_stream.insert(dst, wstream);
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

    pub async fn handle_ingress(s: <T as Transport>::Stream, sender: Sender<Vec<u8>>) {
        
        // a buffed reader

        // let mut reader = 
        //    BufReader::with_capacity(message::MSG_MAXLEN, s);

        let mut msg_reader = message::MessageReader::<T>::new(
            BufReader::with_capacity(message::MSG_MAXLEN, s)
        );

        loop {

            let msg= msg_reader.read_message().await;

            if msg.is_err() {
                warn!("BDN::handle_ingress: {}", msg.unwrap_err());
                continue;
            }

            let msg = msg.unwrap();
            let raw_msg = msg.get_payload();

            info!("BDN::handle_ingress: read {} bytes", raw_msg.len());
            let send_res = sender.send(raw_msg);
            if send_res.is_err() {
                warn!("BDN::handle_ingress: {}", send_res.unwrap_err());
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

    #[async_std::test]
    async fn bdn_1() {

        log::setup_logger().unwrap();
        
        let mut bdn = BDN::<TcpContext>::new(9001).await;

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        bdn.run().await;
    }

    #[async_std::test]
    async fn bdn_2() {

        log::setup_logger().unwrap();
        
        let mut bdn = BDN::<TcpContext>::new(9002).await;

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        // bdn.run().await;
    }

}