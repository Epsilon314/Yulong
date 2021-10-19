pub mod impls;

use crate::route::RouteTable;

use yulong_network::identity::Peer;

/// relay method should provide a callback interface to handle its messages
/// message is defined by concrete implementations, thus declared as bytes.
pub trait RelayCtl {

    fn relay_ctl_callback(route_ctl: &mut RouteTable, sender: &Peer, msg: &[u8])
        -> Vec<(Peer, Vec<u8>)>;

}


/// relay method should provide a callback interface to handle its messages
/// message is defined by concrete implementations, thus declared as bytes.
pub trait PathCtl {

    fn path_ctl_callback(route_ctl: &mut RouteTable, sender: &Peer, msg: &[u8])
        -> Vec<(Peer, Vec<u8>)>;

}