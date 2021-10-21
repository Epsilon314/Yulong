use crate::route::AppLayerRouteUser;
use crate::route_inner::RelayCtl;
use crate::route::RouteTable;
use crate::route::AppLayerRouteInner;

use log::warn;
use yulong_network::identity::Peer;
use super::mlbt_message;
use yulong::utils::AsBytes;

pub struct MlbtRelayCtlContext {

    local_id: u64,
}

impl RelayCtl for MlbtRelayCtlContext {


    fn bootstrap(route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        todo!()
    }

    fn relay_ctl_callback(route_ctl: &mut RouteTable, sender: &Peer,
        msg: &[u8]) -> Vec<(Peer, Vec<u8>)>     
    {
        todo!()
    }

}

impl MlbtRelayCtlContext {

    fn join_callback(route_ctl: &RouteTable, sender: &Peer, msg: &mlbt_message::RelayCtlMessage)
        -> Vec<(Peer, Vec<u8>)>     
    {
        let mut ret: Vec<(Peer, Vec<u8>)> = vec![];
        
        match mlbt_message::RelayMsgJoin::from_bytes(&msg.payload()) {
            Ok(join_msg) => {

                // already have too many links, reject new ones
                if route_ctl.get_relay_count() >= RouteTable::MAX_LINK {
                    ret.push((
                        sender.to_owned(),
                        mlbt_message::RelayMsgReject::new(msg).into_bytes().unwrap()
                    ));
                }

                // accept it
                else {
                    
                    // add relay to route_table
                    // route_ctl.insert_relay(join_msg.src(), from, to);

                    ret.push((
                        sender.to_owned(),
                        mlbt_message::RelayMsgAccept::new(msg).into_bytes().unwrap()
                    ));
                }
            }

            Err(error) => {
                warn!("MlbtRelayCtlContext::join_callback parse RelayMsgJoin failed: {}", error);
                // ignore ill-formed messages 
            }
        }
        ret
    }
    
}