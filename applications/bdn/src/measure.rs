use std::net::SocketAddr;
use std::collections::HashMap;
use yulong_network::identity::Peer;
use async_trait::async_trait;
use log::warn;

// test & update net stat
// update all net stat
// query net stat
#[async_trait]
pub trait NetPref {

    fn latency(&self, to: &Peer) -> Option<u64>;

    fn bandwidth(&self, to: &Peer) -> Option<u64>;

    async fn update(&mut self, target: &Peer);

    async fn update_all(&mut self);
}


// setup known netstat for test simplicity 
pub trait NetStatDebug {
    fn set(&mut self, target: &Peer, addr: Option<SocketAddr>, lat: Option<u64>, bw: Option<u64>);
}


#[derive(Clone, Copy)]
struct NetStatEntry {
    addr: SocketAddr,
    latency: u64,   // ms
    bandwidth: u64, // bps
}


impl NetStatEntry {
    pub fn new(addr: SocketAddr) -> Self {
        Self {
            addr,
            latency: 0,
            bandwidth: 0,
        }
    }
}


#[derive(Clone)]
pub struct NetStat {
    stat_by_peer: HashMap<Peer, NetStatEntry>,
}


#[async_trait]
impl NetPref for NetStat {

    fn latency(&self, to: &Peer) -> Option<u64> {
        match self.stat_by_peer.get(to) {
            Some(v) => Some(v.latency),
            None => {
                warn!("NetStat::latency Unknown peer {}", to);
                None
            }
        }
    }


    fn bandwidth(&self, to: &Peer) -> Option<u64> {
        match self.stat_by_peer.get(to) {
            Some(v) => Some(v.bandwidth),
            None => {
                warn!("NetStat::latency Unknown peer {}", to);
                None
            }
        }
    }


    async fn update(&mut self, target: &Peer) {
        todo!()
    }


    async fn update_all(&mut self, ) {
        todo!()
    }

}


impl NetStatDebug for NetStat {

    fn set(&mut self, target: &Peer, addr: Option<SocketAddr>, lat: Option<u64>, bw: Option<u64>) {
        
        let mut handle = self.stat_by_peer.get_mut(target);
        
        if handle.is_none() {
            if addr.is_none() {
                return;
            }
            self.stat_by_peer.insert(
                target.to_owned(), 
                NetStatEntry::new(addr.unwrap())
            );
            handle = self.stat_by_peer.get_mut(target);
        }

        let handle = handle.unwrap();


        if let Some(lat) = lat {
            handle.latency = lat;
        }

        if let Some(bw) = bw {
            handle.bandwidth = bw;
        }
    }

}


impl NetStat {
    pub fn new() -> Self {
        Self {
            stat_by_peer: HashMap::new(),
        }
    }
}