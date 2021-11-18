use std::{collections::HashMap, hash::Hash};

use yulong_network::identity::{Peer};
use yulong::utils::bidirct_hashmap::BidirctHashmap;

pub trait ParticipantsStore: PartialEq {

    fn new(p: Vec<Peer>) -> Self;

    fn insert(&mut self, p: &Peer, n: u32);

    fn query_pk(&self, p: &mut Peer);

    fn nth(&self, n: u32) -> Option<&Peer>;

    fn get_idx(&self, p: &Peer) -> Option<u32>;
}


pub struct Participants {
    indexed_peer: BidirctHashmap<u32, Peer>
}


// todo: o(n) PartialEq is bad, improve it by changing data struct define?
impl PartialEq for Participants {
    fn eq(&self, other: &Self) -> bool {
        self.indexed_peer == other.indexed_peer
    }
}


impl ParticipantsStore for Participants {
    fn new(pl: Vec<Peer>) -> Self {

        let mut ret = Self {
            indexed_peer: BidirctHashmap::new(),
        };

        for (n, p) in pl.iter().enumerate() {
            ret.indexed_peer.insert(&(n as u32), p);
        }
        
        ret
    }

    fn insert(&mut self, p: &Peer, n: u32) {
        self.insert(p, n)
    }

    fn query_pk(&self, p: &mut Peer) {
        if let Some(idx) = self.indexed_peer.get_by_value(p) {
            let full_peer = self.indexed_peer.get_by_key(idx).unwrap();
            p.set_pubkey(full_peer.pubkey());
        }
    }

    fn nth(&self, n: u32) -> Option<&Peer> {
        self.indexed_peer.get_by_key(&n)
    }

    fn get_idx(&self, p: &Peer) -> Option<u32> {
        self.indexed_peer.get_by_value(p).map(|p| p.to_owned())
    }
}