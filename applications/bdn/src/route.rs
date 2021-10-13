use log::{info, warn, debug};
use yulong_network::{identity::Peer, transport::Transport};
use std::{collections::HashMap, marker::PhantomData};

use crate::common::MessageWithIp;


pub struct Route<T: Transport> {

    transport_type: PhantomData<T>,

    // (src, from) -> [next] 
    relay_table: HashMap<(Peer, Peer), Vec<Peer>>,
    
    // (dst, next)
    route_table: HashMap<Peer, Peer>,
}


impl<T: Transport> Route<T> {

    pub fn new() -> Self {
        Self {
            transport_type: PhantomData::<T>,

            // store the relay network
            relay_table: HashMap::new(),

            // store route
            route_table: HashMap::new(),
        }
    }


    /// Get next hop route to reach dst
    pub fn get_next_hop(&self, dst: &Peer) -> Option<Peer> {
        match self.route_table.get(dst) {
            Some(next_hop) => Some(next_hop.to_owned()),
            None => None,
        }
    }


    /// Get descendent peers to relay to
    pub fn get_relay(&self, src: &Peer, from: &Peer) -> Vec<Peer> {
        match self.relay_table.get(&(src.to_owned(), from.to_owned())) {
            Some(next_hops) => next_hops.to_vec(),
            None => vec![],
        } 
    }


    /// Insert more descendent peer
    pub fn insert_relay(&mut self, src: &Peer, from: &Peer, to: &Peer) {
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
    pub fn remove_relay(&mut self, src: &Peer, from: &Peer, to: &Peer) {
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


    pub fn insert_route(&mut self, dst: &Peer, next: &Peer) {
        if let Some(handle) = self.route_table.get_mut(dst) {
            warn!("Route::insert_route Overwrite the route to {}. Changed from {} to {}", dst, handle, next);
            *handle = next.to_owned();
        }
        else {
            self.route_table.insert(dst.to_owned(), next.to_owned());
        }
    }


    pub fn remove_route(&mut self, dst: &Peer) {
        if self.route_table.remove(dst).is_none() {
            warn!("Route::remove_route Try to remove the route to {}, which do not exist.", dst);
        }
        else {
            debug!("Route::remove_route Remove route to {}", dst);
        }
    }


    // todo: accept a route related command; apply some changes; and return reaction
    pub fn handle_route_message(msg: &MessageWithIp) {
        unimplemented!()
    }

}


#[cfg(test)]
mod test {
    use super::*;
    use yulong_tcp::TcpContext;
    use yulong::log;

    #[test]
    fn insert_remove_relay() {
        // log::setup_logger("relay_test").unwrap();

        let mut route = Route::<TcpContext>::new();
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
        let mut route = Route::<TcpContext>::new();
        let p1 = Peer::from_bytes(&[1]);
        let p2 = Peer::from_bytes(&[2]);
        let p3 = Peer::from_bytes(&[3]);
        let p4 = Peer::from_bytes(&[4]);
        route.insert_route(&p4, &p3);
        route.insert_route(&p3, &p3);
        route.insert_route(&p2, &p3);
        route.insert_route(&p1, &p3);

        let next = route.get_next_hop(&p1);
        assert_eq!(next.unwrap(), p3);

        route.remove_route(&p3);
        let next = route.get_next_hop(&p3);
        assert!(next.is_none());
    }
}