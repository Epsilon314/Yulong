use futures::{AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
    type_alias::FutureRet,
};

use yulong_network::{
    identity::{Peer, Me}, 
    transport::{Transport},
};

use std::{
    net::{
        SocketAddr, IpAddr, Ipv4Addr,
    }, 
    collections::HashMap,
};

use crate::route::Route;
use async_std::task;

pub struct BDN<T: Transport> {

    pub local_identity: Me,
    pub listener: <T as Transport>::Listener,
    
    pub address_book: BidirctHashmap<Peer, SocketAddr>,
    pub established_stream: HashMap<Peer, <T as Transport>::Stream>,

    // route: Route,

    income_handler: Option<for<'a> fn(&BDN<T>, &'a [u8]) -> FutureRet<'a, bool>>,
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
            
            established_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
            
            income_handler: None,
        }
    }

    // accept incoming connections and spawn tasks to serve them
    pub async fn run(&mut self) {

        match T::accept(&mut self.listener).await {
            
            Ok(istream) => {
                // a new incoming connection
                
                // If it is a new peer, add it to the address_book with a temp
                // peer id derived from provided public key.
                if !self.address_book.contains_value(&istream.remote_addr) {
                    self.address_book.insert(
                        Peer::from_public_key(&istream.remote_pk),
                        istream.remote_addr.clone());
                }

                if let Some(f) = self.income_handler {
                    f(self, &[1,2,3]);
                    // async_std::task::spawn(
                    //     self.income_handler
                    // );
                }
                else {
                    // Todo: log no handle method error after Log module is introduced
                }
            }

            Err(error) => {
                // Todo: log connection error after Log module is introduced
            }
        }
    }

    pub async fn connect(&mut self) {

        for (peer, addr) in self.address_book.iter() {
            if self.established_stream.contains_key(peer) {
                continue;
            }
            match T::connect(addr).await {
                Ok(stream) => {
                    self.established_stream.insert(peer.clone(), stream);
                }
                Err(error) => {
                    // Todo: log connection error after Log module is introduced
                }
            }
        }
    }

    pub async fn send_to(&mut self, dst: Peer, msg: &[u8]) {
        let wstream = self.established_stream.get_mut(&dst).unwrap();
        wstream.write_all(msg).await.unwrap();
    }

    pub async fn broadcast(&mut self, src: Peer, msg: &[u8]) {
        unimplemented!()
    }

    pub fn register_msg_handler(&mut self, f: for<'a> fn(&BDN<T>, &'a [u8]) -> FutureRet<'a, bool>) {
        self.income_handler = Some(f);
    }
}


#[cfg(test)]
mod test {
    use super::BDN;
    use yulong_tcp::TcpContext;
    use async_std::{self, future};

    async fn test_handle(bdn: &BDN<TcpContext>, buf: &[u8]) -> bool {
        future::ready(true).await
    }

    #[async_std::test]
    async fn new_bdn() {
        let bdn = BDN::<TcpContext>::new(10001).await;
        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        assert!(true);
    }
}