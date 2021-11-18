use std::collections::HashMap;
use yulong_network::identity::Peer;


// query and update mlbt stat
// mlbt stat:
// src_inv: interval between root start -> finish relaying
// relay_inv: interval between recv -> all desc recv
pub trait MlbtStatMaintainer {

    fn src_inv(&self, tr: &Peer) -> Option<u64>;

    fn relay_inv(&self, tr: &Peer) -> Option<u64>;

    fn merge_thrd(&self, tr: &Peer) -> Option<u64>;

    fn insert_default(&mut self, tr: &Peer);

    // todo: 
    
    // src_inv update cb
    // init src_inv update (root node only)

    // relay_inv update cb
    // init relay_inv update
    
}


// directly modify mlbt stat for test and debug simplicity
pub trait MlbtStatDebug {
    
    fn set_src_inv(&mut self, tr: &Peer, _: u64) -> Option<()>;
    
    fn set_relay_inv(&mut self, tr: &Peer, _: u64) -> Option<()>;

}


struct MlbtStat {
    src_inv: u64,
    relay_inv: u64,
    merge_thrd: u64,
}


pub struct MlbtStatList {
    inner_list: HashMap<Peer, MlbtStat>
}


impl MlbtStat {

    pub fn new() -> Self {
        Self {
            // todo
            src_inv: 0,
            relay_inv: 0,
            merge_thrd: 500,    // never set to zero
        }
    }

}


impl MlbtStatList {

    pub fn new() -> Self {
        Self {
            inner_list: HashMap::new()
        }
    }
}


impl MlbtStatMaintainer for MlbtStatList {
    fn src_inv(&self, tr: &Peer) -> Option<u64> {
        match self.inner_list.get(tr) {
            Some(stat) => Some(stat.src_inv),
            None => None,
        }
    }

    fn relay_inv(&self, tr: &Peer) -> Option<u64> {
        match self.inner_list.get(tr) {
            Some(stat) => Some(stat.relay_inv),
            None => None,
        }
    }

    fn merge_thrd(&self, tr: &Peer) -> Option<u64> {
        match self.inner_list.get(tr) {
            Some(stat) => Some(stat.merge_thrd),
            None => None,
        }
    }

    fn insert_default(&mut self, tr: &Peer) {
        todo!()
    }
}


impl MlbtStatDebug for MlbtStatList {
    fn set_src_inv(&mut self, tr: &Peer, new_value: u64) -> Option<()> {
        match self.inner_list.get_mut(tr) {
            Some(stat) => {
                stat.src_inv = new_value;
                Some(())
            }
            None => {
                None
            }
        }
    }

    fn set_relay_inv(&mut self, tr: &Peer, new_value: u64) -> Option<()> {
        match self.inner_list.get_mut(tr) {
            Some(stat) => {
                stat.relay_inv = new_value;
                Some(())
            }
            None => {
                None
            }
        }
    }
}