use futures::AsyncWriteExt;
use yulong_network::{identity::{Peer, Me}, transport::{Transport}};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use crate::route::Route;
use std::collections::HashMap;

pub struct BDN<T: Transport> {

    pub local_identity: Me,
    pub listener: <T as Transport>::Listener,
    
    pub address_book: HashMap<Peer, SocketAddr>,
    
    pub established_stream: HashMap<Peer, <T as Transport>::Stream>,

    // route: Route,
}

impl<T: Transport> BDN<T> {
    
    pub async fn new(port: u16) -> Self {
        Self {
            local_identity: Me::new(),

            listener: 
                T::listen(
                    &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
                ).await.ok().unwrap(),
            
            address_book: HashMap::<Peer, SocketAddr>::new(),
            
            established_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
        }
    }

    // accept incoming connections and spawn tasks to serve them
    pub async fn run(&mut self) {

        match T::accept(&mut self.listener).await {
            Ok(istream) => {
                // a new incoming connection

            }
            Err(error) => {
                // Todo: log connection error after Log module is introduced
            }
        }
    }

    pub async fn connect(&mut self) -> Self {

        for (peer, addr) in self.address_book.iter() {
            match T::connect(addr).await {
                Ok(stream) => {
                    self.established_stream.insert(peer.clone(), stream);
                }
                Err(error) => {
                    // Todo: log connection error after Log module is introduced
                }
            }
        }
        unimplemented!()
    }

    pub async fn send_to(&mut self, dst: Peer, msg: &[u8]) {
        let wstream = self.established_stream.get_mut(&dst).unwrap();
        wstream.write_all(msg).await.unwrap();
    }

    pub async fn broadcast(&mut self, src: Peer, msg: &[u8]) {
        unimplemented!()
    }
}


#[cfg(test)]
mod test {
    use super::BDN;
    use yulong_tcp::TcpContext;

    #[async_std::test]
    async fn new_bdn() {
        let bdn = BDN::<TcpContext>::new(10001).await;
        println!("New BDN client at: {:?}", bdn.local_identity.raw_id);
        assert!(true);
    }
}