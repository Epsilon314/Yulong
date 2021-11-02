use std::net::IpAddr;
use std::collections::HashMap;
use yulong_network::identity::Peer;
use async_trait::async_trait;

// test & update net stat
// update all net stat
// query net stat
#[async_trait]
pub trait NetPref {

    fn latency(&self, to: &Peer) -> f32;

    fn bandwidth(&self, to: &Peer) -> f32;

    async fn update(&mut self, target: &Peer);

    async fn update_all(&mut self);
}


// setup known netstat for test simplicity 
pub trait NetStatDebug {
    fn set(target: &Peer, lat: Option<f32>, bw: Option<f32>);
}


#[derive(Clone, Copy)]
struct NetStatEntry {
    addr: IpAddr,
    latency: f32,   // ms
    bandwidth: f32, // Mbps
}


#[derive(Clone, Copy)]
pub struct NetStat {
}


#[async_trait]
impl NetPref for NetStat {

    fn latency(&self, to: &Peer) -> f32 {
        todo!()
    }


    fn bandwidth(&self, to: &Peer) -> f32 {
        todo!()
    }


    async fn update(&mut self, target: &Peer) {
        todo!()
    }


    async fn update_all(&mut self, ) {
        todo!()
    }

}


impl NetStatDebug for NetStat {

    fn set(target: &Peer, lat: Option<f32>, bw: Option<f32>) {
        todo!()
    }

}


impl NetStat {
    pub fn new() -> Self {
        Self {}
    }
}