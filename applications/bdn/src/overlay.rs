use futures::{AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
    type_alias::BoxedFuture,
};

use yulong_network::{
    identity::{Peer, Me}, 
    transport::{Transport},
};

use std::{collections::HashMap, net::{
        SocketAddr, IpAddr, Ipv4Addr,
    }, sync::{Arc, Mutex}};

use crate::route::Route;
use async_std::task;

pub struct BDN<T: Transport> {

    pub local_identity: Me,
    pub listener: <T as Transport>::Listener,
    
    pub address_book: BidirctHashmap<Peer, SocketAddr>,
    pub established_stream: Arc<Mutex<HashMap<Peer, <T as Transport>::Stream>>>,

    // route: Route,

    income_handler: Option<fn(&'_ [u8]) -> BoxedFuture<'_, bool>>,
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
            
            established_stream: Arc::new(Mutex::new(HashMap::<Peer, <T as Transport>::Stream>::new())),
            
            income_handler: None,
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

                let mut streams = self.established_stream.lock().unwrap();
                streams.insert(peer, istream.stream);                
                true
            }

            Err(error) => {
                // Todo: log connection error after Log module is introduced
                true
            }
        } {}
    }


    pub async fn connect(&mut self) {

        for (peer, addr) in self.address_book.iter() {
            let mut stream_handle = self.established_stream.lock().unwrap();
            if stream_handle.contains_key(peer) {
                continue;
            }
            match T::connect(addr).await {
                Ok(stream) => {
                    stream_handle.insert(peer.clone(), stream);
                }
                Err(error) => {
                    // Todo: log connection error after Log module is introduced
                }
            }
        }
    }

    pub async fn send_to(&self, dst: Peer, msg: &[u8]) {
        let mut stream_handle = self.established_stream.lock().unwrap();
        let wstream = stream_handle.get_mut(&dst).unwrap();
        wstream.write_all(msg).await.unwrap();
    }

    pub async fn broadcast(&self, src: Peer, msg: &[u8]) {
        unimplemented!()
    }

    pub fn register_msg_handler(&mut self, f: fn(&'_ [u8]) -> BoxedFuture<'_, bool>) {
        self.income_handler = Some(f);
    }
}


#[cfg(test)]
mod test {
    use super::BDN;
    use yulong_tcp::TcpContext;
    use async_std::{self, future};
    use yulong::utils::type_alias::BoxedFuture;

    async fn test_handle(buf: &[u8]) -> bool {
        future::ready(buf == [1,2,3,4]).await
    }

    fn boxed_handle(buf: &'_ [u8]) -> BoxedFuture<'_, bool> {
        Box::pin(test_handle(buf))
    }


    #[async_std::test]
    async fn new_bdn() {
        
        let mut bdn = BDN::<TcpContext>::new(10001).await;

        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        
        bdn.register_msg_handler(boxed_handle);
        if let Some(f) = bdn.income_handler {
           assert_eq!(true, f(&[1,2,3,4]).await);
        }
        else {
            assert!(false);
        }        
    }
}