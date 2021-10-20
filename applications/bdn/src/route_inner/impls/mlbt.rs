use crate::route_inner::RelayCtl;
use crate::route::RouteTable;
use crate::route::AppLayerRouteInner;

use yulong_network::identity::Peer;
use super::mlbt_message;

pub struct MlbtRelayCtlContext {
    local_id: u64,
}

impl RelayCtl for MlbtRelayCtlContext {


    fn bootstrap(route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        todo!()
    }

    fn relay_ctl_callback(
        route_ctl: &mut RouteTable,
        sender: &Peer,
        msg: &[u8]) -> Vec<(Peer, Vec<u8>)>     
    {
        todo!()
    }

}

impl MlbtRelayCtlContext {
    fn join_callback(sender: &Peer) -> Vec<(Peer, Vec<u8>)> {
        let ret: Vec<(Peer, Vec<u8>)> = vec![];
        
        todo!("config a max_link");
        
        ret
    }
}