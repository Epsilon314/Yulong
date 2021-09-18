use futures::AsyncWriteExt;
use yulong_network::{identity::{Peer, Me}, transport::Transport};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use crate::route::Route;
use std::collections::HashMap;

pub struct BDN<T: Transport> {

    pub local_identity: Me,
    pub listener: <T as Transport>::Listener,
    pub egress_stream: HashMap<Peer, <T as Transport>::Stream>

    // route: Route,
}

impl<T: Transport> BDN<T> {
    
    pub async fn new(port: u16) -> Self {
        Self {
            local_identity: Me::new(),

            listener: 
                T::listen(
                    SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port)
                ).await.ok().unwrap(),
            
            egress_stream: HashMap::<Peer, <T as Transport>::Stream>::new()
        }
    }

    pub async fn connect() -> Self {
        unimplemented!()
    }

    pub async fn send_to(&mut self, dst: Peer, msg: &[u8]) {
        let wstream = self.egress_stream.get_mut(&dst).unwrap();
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