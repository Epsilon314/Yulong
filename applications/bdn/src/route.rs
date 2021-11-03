use log::{info, warn, debug};
use num_traits::ToPrimitive;
use yulong::utils::bidirct_hashmap::BidirctHashmap;
use yulong_network::{identity::Peer};
use std::hash::Hash;
use std::{collections::HashMap, fmt::Debug};

use async_trait::async_trait;

use crate::msg_header::{MsgHeader, MsgType, MsgTypeKind, RelayMethodKind};

use crate::{
    common::MessageWithIp,
    message::OverlayMessage,
    route_inner::RelayCtl,
    measure::NetStat,
    measure::NetPref,
};

/// Application-layer route user interface, query only
pub trait AppLayerRouteUser {

    type Host: Clone + Debug;

    fn get_delegate(&self, src: &Self::Host) -> Option<Self::Host>;

    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host>;

    fn get_relay(&self, src: &Self::Host) -> Vec<Self::Host>;

}


/// Application-layer route manage interface, insert, delete and query
pub trait AppLayerRouteInner: AppLayerRouteUser {

    fn insert_path(&mut self, dst: &Self::Host, next: &Self::Host);

    fn remove_path(&mut self, dst: &Self::Host);

    fn reg_delegate(&mut self, src: &Self::Host, del: &Self::Host);

    fn insert_relay(&mut self, src: &Self::Host, to: &Self::Host);

    fn remove_relay(&mut self, src: &Self::Host, to: &Self::Host);

    fn get_relay_count(&self) -> u32;

    fn get_relay_count_by_tree(&self, src: &Self::Host) -> u32;

}


pub struct Route<R: RelayCtl> {

    route_table: RouteTable,

    relay_mod: R,

    netstat: NetStat,
}

pub struct RouteTable {

    local_id: Peer,

    delegates: BidirctHashmap<Peer, Peer>,

    // src -> [next] 
    relay_table: HashMap<Peer, Vec<Peer>>,
    
    // dst -> next
    path_table: HashMap<Peer, Peer>,

    relay_counter: u32,
    relay_ct_per_tree: HashMap<Peer, u32>,
}

impl RouteTable {

    pub const MAX_LINK: u32 = 128;

    const DEL_NUM: u32 = 1;

    pub fn new(local: &Peer) -> Self {
        Self {

            local_id: local.to_owned(),

            delegates: BidirctHashmap::new(),

            // store the relay network
            relay_table: HashMap::new(),

            // store route
            path_table: HashMap::new(),
            
            relay_counter: 0,
            relay_ct_per_tree: HashMap::new(),
        }
    }


    pub fn local_id(&self) -> Peer {
        self.local_id.clone()
    }
}

impl<R: RelayCtl> Route<R> {

    pub fn new(local: &Peer) -> Self {
        Self {
            route_table: RouteTable::new(local),

            relay_mod: R::new(),

            netstat: NetStat::new(),
        }
    }


    // accept a route related command; apply some changes; and return reaction
    pub fn handle_route_message(&mut self, msg: &OverlayMessage) -> Vec<OverlayMessage> {

        // Pack all messages to be sent and return it to bdn
        // BDN decides when & how to send them, do not assume the order of transmission
        let mut reply_list = Vec::<OverlayMessage>::new();

        let ctl_msgs = self.relay_mod.relay_ctl_callback(&mut self.route_table, &msg.from(), &msg.payload());
        
        for (peer, payload) in ctl_msgs {
            let packed_message = OverlayMessage::new(
                MsgHeader::build(
                    MsgTypeKind::ROUTE_MSG, 
                    false, 
                    RelayMethodKind::LOOKUP_TABLE_1, 
                    0, 
                    0
                ).unwrap(),
                
               
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


    // no incoming message, invoked temporally
    pub fn invoke_heartbeat(&self) -> Vec<OverlayMessage> {
        let mut ret = Vec::<OverlayMessage>::new();

        let ctl_msgs = self.relay_mod.heartbeat(&self.route_table);
        for (peer, payload) in ctl_msgs {
            let packed_message = OverlayMessage::new(
                MsgHeader::build(
                    MsgTypeKind::ROUTE_MSG, 
                    false, 
                    RelayMethodKind::LOOKUP_TABLE_1, 
                    0, 
                    0
                ).unwrap(),

                // to be filled by caller 
                &Peer::BROADCAST_ID,

                // to be filled by caller 
                &Peer::BROADCAST_ID,

                &peer,

                &payload
            );
            ret.push(packed_message);
        }
        ret
    }


    pub fn relay_receipt(&mut self, all_success: bool) {
        self.relay_mod.relay_receipt(&mut self.route_table, all_success);
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
    fn get_relay(&self, src: &Self::Host) -> Vec<Self::Host> {
        match self.relay_table.get(src) {
            Some(next_hops) => next_hops.to_vec(),
            None => vec![],
        }
    }

    
    fn get_delegate(&self, src: &Self::Host) -> Option<Self::Host> {
        self.delegates.get_by_key(src).map(|p| p.to_owned())
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
    fn insert_relay(&mut self, src: &Self::Host, to: &Self::Host) {
        
        self.relay_counter += 1;

        match self.relay_table.get_mut(src) {
            Some(handle) => {
                if handle.contains(to) {
                    warn!("Route::insert_relay already exists: {} -> {} ", src, to);
                    self.relay_counter -= 1;
                    return;
                }
                else {
                    handle.push(to.to_owned());
                }
            }
            None => {
                self.relay_table.insert(
                    src.to_owned(), 
                    vec![to.to_owned()]
                );
            }
        }


        // todo: fix counter
        match self.relay_ct_per_tree.get_mut(src) {
            Some(ct_handle) => {
                *ct_handle += 1;
            }
            None => {
                self.relay_ct_per_tree.insert(src.to_owned(), 1);
            }
        }

        debug!("Route::insert_relay {} -> {}", src, to);
    }


    /// Remove a descendent peer, fail silently
    fn remove_relay(&mut self, src: &Self::Host, to: &Self::Host) {
        match self.relay_table.get_mut(src) {
            Some(handle) => {
                // remove all matching items
                handle.retain(|x| x != to);
                debug!("Route::remove_relay {} -> {}", src, to);
            }
            None => {
                warn!("Route::remove_relay {} -> {}, no matching key", src, to);
            }
        }

        self.relay_counter -= 1;

        match self.relay_ct_per_tree.get_mut(src) {
            Some(ct_handle ) => {
                if *ct_handle == 0 {
                    warn!("Route::remove_relay total link counter is inconsistent");
                }
                else {
                    *ct_handle -= 1;
                }
            }
            None => {
                warn!("Route::remove_relay link counter for {} not found", src);
            }
        }
    }


    fn get_relay_count(&self) -> u32 {
        self.relay_counter
    }


    fn get_relay_count_by_tree(&self, src: &Self::Host) -> u32 {
        *self.relay_ct_per_tree.get(src).unwrap_or(&0)
    }


    fn reg_delegate(&mut self, src: &Self::Host, del: &Self::Host) {
        if let Some(d) = self.delegates.get_by_key(src) {
            warn!("Route::reg_delegate register delegate conflict")
        }
        else {
            self.delegates.insert(src, del);
            debug!("Route::reg_delegate register new delegate {} for {}", del, src);
        }
    }
    
}

impl<R: RelayCtl> AppLayerRouteUser for Route<R> {
    type Host = Peer;

    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host> {
        self.route_table.get_next_hop(dst)
    }

    fn get_relay(&self, src: &Self::Host) -> Vec<Self::Host> {
        self.route_table.get_relay(src)
    }

    fn get_delegate(&self, src: &Self::Host) -> Option<Self::Host> {
        self.route_table.get_delegate(src)
    }
}


impl<R: RelayCtl> AppLayerRouteInner for Route<R> {
    fn insert_path(&mut self, dst: &Self::Host, next: &Self::Host) {
        self.route_table.insert_path(dst, next)
    }

    fn remove_path(&mut self, dst: &Self::Host) {
        self.route_table.remove_path(dst)
    }

    fn insert_relay(&mut self, src: &Self::Host, to: &Self::Host) {
        self.route_table.insert_relay(src, to)
    }

    fn remove_relay(&mut self, src: &Self::Host, to: &Self::Host) {
        self.route_table.remove_relay(src, to)
    }

    fn get_relay_count(&self) -> u32 {
        self.route_table.get_relay_count()
    }

    fn get_relay_count_by_tree(&self, src: &Self::Host) -> u32 {
        self.route_table.get_relay_count_by_tree(src)
    }

    fn reg_delegate(&mut self, src: &Self::Host, del: &Self::Host) {
        self.route_table.reg_delegate(src, del)
    }
}


// wrap it for calling convenient

#[async_trait]
impl<R: RelayCtl> NetPref for Route<R> {

    fn latency(&self, to: &Peer) -> f32 {
        self.netstat.latency(to)
    }


    fn bandwidth(&self, to: &Peer) -> f32 {
        self.netstat.bandwidth(to)
    }


    async fn update(&mut self, target: &Peer) {
        self.netstat.update(target).await;
    }


    async fn update_all(&mut self) {
        self.netstat.update_all().await;
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use crate::route_inner::impls::mlbt::MlbtRelayCtlContext;
    use yulong::log;

    #[test]
    fn insert_remove_relay() {
        log::setup_logger("relay_test").unwrap();

        let p1 = Peer::from_bytes(&[1]);
        let p2 = Peer::from_bytes(&[2]);
        let p3 = Peer::from_bytes(&[3]);
        let p4 = Peer::from_bytes(&[4]);

        let mut route = Route::<MlbtRelayCtlContext>::new(&p1);

        route.insert_relay(&p1, &p3);
        route.insert_relay(&p1, &p4);
        
        let rl1 = route.get_relay(&p1);
        assert_eq!(rl1, vec![p3.clone(), p4.clone()]);
        
        route.remove_relay(&p1, &p3);
        route.remove_relay(&p1, &p3);
        let rl1 = route.get_relay(&p1);
        assert_eq!(rl1, vec![p4.clone()]);
    }


    #[test]
    fn insert_remove_route() {
        log::setup_logger("route_test").unwrap();

        let p1 = Peer::from_bytes(&[1]);
        let p2 = Peer::from_bytes(&[2]);
        let p3 = Peer::from_bytes(&[3]);
        let p4 = Peer::from_bytes(&[4]);

        let mut route = Route::<MlbtRelayCtlContext>::new(&p1);

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