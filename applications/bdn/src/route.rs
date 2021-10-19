use log::{info, warn, debug};
use num_traits::ToPrimitive;
use yulong_network::{identity::Peer};
use std::{collections::HashMap, fmt::Debug};
use std::marker::PhantomData;

use crate::msg_header::{MsgType, MsgTypeKind};
use crate::{
    common::MessageWithIp,
    message::OverlayMessage,
    route_inner::RelayCtl,
};

/// Application-layer route user interface, query only
pub trait AppLayerRouteUser {

    type Host: Clone + Debug;

    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host>;

    fn get_relay(&self, src: &Self::Host, from: &Self::Host) -> Vec<Self::Host>;

}


/// Application-layer route manage interface, insert, delete and query
pub trait AppLayerRouteInner: AppLayerRouteUser {

    fn insert_path(&mut self, dst: &Self::Host, next: &Self::Host);

    fn remove_path(&mut self, dst: &Self::Host);

    fn insert_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host);

    fn remove_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host);

}

pub struct Route<R: RelayCtl> {

    route_table: RouteTable,

    relay_method_in_use: PhantomData<R>,
}

pub struct RouteTable {

    // (src, from) -> [next] 
    relay_table: HashMap<(Peer, Peer), Vec<Peer>>,
    
    // (dst, next)
    path_table: HashMap<Peer, Peer>,
}

impl RouteTable {
    pub fn new() -> Self {
        Self {
            // store the relay network
            relay_table: HashMap::new(),

            // store route
            path_table: HashMap::new(),
        }
    }
}

impl<R: RelayCtl> Route<R> {

    pub fn new() -> Self {
        Self {
            route_table: RouteTable::new(),

            relay_method_in_use: PhantomData::<R>,
        }
    }


    // todo: accept a route related command; apply some changes; and return reaction
    pub fn handle_route_message(&mut self, msg: &OverlayMessage) -> Vec<OverlayMessage> {

        // Pack all messages to be sent and return it to bdn
        // BDN decides when & how to send them, do not assume the order of transmission
        let mut reply_list = Vec::<OverlayMessage>::new();

        let ctl_msgs = 
            R::relay_ctl_callback(&mut self.route_table, &msg.from(), &msg.payload());
        
        for (peer, payload) in ctl_msgs {
            let packed_message = OverlayMessage::new(
                ToPrimitive::to_u32(&MsgTypeKind::ROUTE_MSG).unwrap(),
                
                // to be filled by caller 
                &Peer::BROADCAST_ID,

                // to be filled by caller 
                &Peer::BROADCAST_ID,
                &peer,
                &payload
            );
            reply_list.push(packed_message);
        }

        reply_list
    }

}


impl AppLayerRouteUser for RouteTable {

    type Host = Peer;


    /// Get next hop route to reach dst
    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host> {
        match self.path_table.get(dst) {
            Some(next_hop) => Some(next_hop.to_owned()),
            None => None,
        }
    }


    /// Get descendent peers to relay to
    fn get_relay(&self, src: &Self::Host, from: &Self::Host) -> Vec<Self::Host> {
        match self.relay_table.get(&(src.to_owned(), from.to_owned())) {
            Some(next_hops) => next_hops.to_vec(),
            None => vec![],
        }
    }

}

impl AppLayerRouteInner for RouteTable {
    
    fn insert_path(&mut self, dst: &Self::Host, next: &Self::Host) {
        if let Some(handle) = self.path_table.get_mut(dst) {
            warn!("Route::insert_route Overwrite the route to {}. Changed from {} to {}", dst, handle, next);
            *handle = next.to_owned();
        }
        else {
            self.path_table.insert(dst.to_owned(), next.to_owned());
        }
    }


    fn remove_path(&mut self, dst: &Self::Host) {
        if self.path_table.remove(dst).is_none() {
            warn!("Route::remove_route Try to remove the route to {}, which do not exist.", dst);
        }
        else {
            debug!("Route::remove_route Remove route to {}", dst);
        }
    }


    /// Insert more descendent peer
    fn insert_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host) {
        match self.relay_table.get_mut(&(src.to_owned(), from.to_owned())) {
            Some(handle) => {
                if handle.contains(to) {
                    warn!("Route::insert_relay already exists: ({}, {}) -> {} ", src, from, to);
                    return;
                }
                else {
                    handle.push(to.to_owned());
                }
            }
            None => {
                self.relay_table.insert(
                    (src.to_owned(), from.to_owned()), 
                    vec![to.to_owned()]
                );
            }
        }
        debug!("Route::insert_relay ({}, {}) -> {}", src, from, to);
    }


    /// Remove a descendent peer, fail silently
    fn remove_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host) {
        match self.relay_table.get_mut(&(src.to_owned(), from.to_owned())) {
            Some(handle) => {
                // remove all matching items
                handle.retain(|x| x != to);
                debug!("Route::remove_relay ({}, {}) -> {}", src, from, to);
            }
            None => {
                warn!("Route::remove_relay ({}, {}) -> {}, no matching key", src, from, to);
            }
        }
    }
    
}

impl<R: RelayCtl> AppLayerRouteUser for Route<R> {
    type Host = Peer;

    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host> {
        self.route_table.get_next_hop(dst)
    }

    fn get_relay(&self, src: &Self::Host, from: &Self::Host) -> Vec<Self::Host> {
        self.route_table.get_relay(src, from)
    }
}


impl<R: RelayCtl> AppLayerRouteInner for Route<R> {
    fn insert_path(&mut self, dst: &Self::Host, next: &Self::Host) {
        self.route_table.insert_path(dst, next)
    }

    fn remove_path(&mut self, dst: &Self::Host) {
        self.route_table.remove_path(dst)
    }

    fn insert_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host) {
        self.route_table.insert_relay(src, from, to)
    }

    fn remove_relay(&mut self, src: &Self::Host, from: &Self::Host, to: &Self::Host) {
        self.route_table.remove_relay(src, from, to)
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::route_inner::impls::mlbt::MlbtRelayCtlContext;

    #[test]
    fn insert_remove_relay() {
        // log::setup_logger("relay_test").unwrap();

        let mut route = Route::<MlbtRelayCtlContext>::new();
        let p1 = Peer::from_bytes(&[1]);
        let p2 = Peer::from_bytes(&[2]);
        let p3 = Peer::from_bytes(&[3]);
        let p4 = Peer::from_bytes(&[4]);
        route.insert_relay(&p1, &p2, &p3);
        route.insert_relay(&p1, &p2, &p4);
        
        let rl1 = route.get_relay(&p1, &p2);
        assert_eq!(rl1, vec![p3.clone(), p4.clone()]);
        
        route.remove_relay(&p1, &p2, &p3);
        route.remove_relay(&p1, &p2, &p3);
        let rl1 = route.get_relay(&p1, &p2);
        assert_eq!(rl1, vec![p4.clone()]);
    }


    #[test]
    fn insert_remove_route() {
        // log::setup_logger("route_test").unwrap();
        let mut route = Route::<MlbtRelayCtlContext>::new();
        let p1 = Peer::from_bytes(&[1]);
        let p2 = Peer::from_bytes(&[2]);
        let p3 = Peer::from_bytes(&[3]);
        let p4 = Peer::from_bytes(&[4]);
        route.insert_path(&p4, &p3);
        route.insert_path(&p3, &p3);
        route.insert_path(&p2, &p3);
        route.insert_path(&p1, &p3);

        let next = route.get_next_hop(&p1);
        assert_eq!(next.unwrap(), p3);

        route.remove_path(&p3);
        let next = route.get_next_hop(&p3);
        assert!(next.is_none());
    }
}