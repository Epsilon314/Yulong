use yulong_network::{identity::Peer, transport::Transport};
use std::{collections::HashMap, marker::PhantomData};

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
            relay_table: HashMap::new(),
            route_table: HashMap::new(),
        }
    }

    pub fn next_hop(&self, dst: &Peer) -> Option<Peer> {
        match self.route_table.get(dst) {
            Some(next_hop) => Some(next_hop.to_owned()),
            None => None,
        }
    }

    pub fn next_hops(&self, src: &Peer, from: &Peer) -> Vec<Peer> {
        match self.relay_table.get(&(src.to_owned(), from.to_owned())) {
            Some(next_hops) => next_hops.to_vec(),
            None => vec![],
        } 
    }
}


#[cfg(test)]
mod test {
    use super::Route;
}