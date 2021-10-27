pub mod impls;

use crate::route::RouteTable;

use yulong_network::identity::Peer;

/// Relay method should provide a callback interface to handle its messages,
/// and a bootstrap function to generate initial messages according to initial
/// RouteTable configurations.
/// 
/// message is defined by concrete implementations, thus declared as bytes.
pub trait RelayCtl: Send {

    fn new() -> Self;

    // will not modify RouteTable 
    fn heartbeat(&self, route_ctl: &RouteTable) -> Vec<(Peer, Vec<u8>)>;

    fn bootstrap(&mut self, route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)>;

    fn relay_ctl_callback(&mut self, route_ctl: &mut RouteTable, sender: &Peer, msg: &[u8])
        -> Vec<(Peer, Vec<u8>)>;

}


/// Relay method should provide a callback interface to handle its messages,
/// and a bootstrap function to generate initial messages according to initial
/// RouteTable configurations.
/// 
/// message is defined by concrete implementations, thus declared as bytes.
pub trait PathCtl {

    fn new() -> Self;

    fn bootstrap(route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)>;

    fn path_ctl_callback(route_ctl: &mut RouteTable, sender: &Peer, msg: &[u8])
        -> Vec<(Peer, Vec<u8>)>;

}