use crate::route_inner::RelayCtl;
use crate::route::RouteTable;
use crate::route::AppLayerRouteInner;

use yulong_network::identity::Peer;


pub struct MlbtRelayCtlContext {

}

impl RelayCtl for MlbtRelayCtlContext {

    fn relay_ctl_callback(route_ctl: &mut RouteTable, sender: &Peer, msg: &[u8])
        -> Vec<(Peer, Vec<u8>)> {

        

        todo!()
    }
}