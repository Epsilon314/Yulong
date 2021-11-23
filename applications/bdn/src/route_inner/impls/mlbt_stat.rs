use std::collections::HashMap;
use yulong_network::identity::Peer;

const DELAY_AVERAGE_WD: u32 = 10;

// query and update mlbt stat
// mlbt stat:

// src_inv: interval between root start -> self finish relaying
// relay_inv: interval between recv -> all desc recv
pub trait MlbtStatMaintainer {

    fn src_inv(&self, tr: &Peer) -> Option<u64>;
    fn relay_inv(&self, tr: &Peer) -> Option<u64>;

    fn src_inv_desc(&self, tr: &Peer, query: &Peer) -> Option<u64>;
    fn relay_inv_desc(&self, tr: &Peer, query: &Peer) -> Option<u64>;

    // moving average delay by timestamping
    fn delay_ts(&self, peer: &Peer) -> Option<u64>;

    fn merge_thrd(&self, tr: &Peer) -> Option<u64>;

    fn insert_default(&mut self, tr: &Peer);


    fn roll_update_delay_ts(&mut self, peer: &Peer, new_delay: u64);
     
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

    fn set_src_inv_desc(&mut self, tr: &Peer, query: &Peer, _: u64) -> Option<()>;

    fn set_relay_inv_desc(&mut self, tr: &Peer, query: &Peer, _: u64) -> Option<()>;

    fn set_delay_ts(&mut self, query: &Peer, _: u64) -> Option<()>;

}


// self stat 
struct MlbtStat {
    src_inv: u64,
    relay_inv: u64,
    merge_thrd: u64,
}


struct NeighbourStat {
    delay_ts: u64,
}


pub struct MlbtStatList {

    // self stat in each tree
    inner_list: HashMap<Peer, MlbtStat>,

    neighbour_list: HashMap<Peer, NeighbourStat>,

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
            inner_list: HashMap::new(),
            neighbour_list: HashMap::new(),
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

    fn src_inv_desc(&self, tr: &Peer, query: &Peer) -> Option<u64> {
        todo!()
    }

    
    fn relay_inv_desc(&self, tr: &Peer, query: &Peer) -> Option<u64> {
        todo!()
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


    fn delay_ts(&self, peer: &Peer) -> Option<u64> {
        match self.neighbour_list.get(peer) {
            Some(stat) => Some(stat.delay_ts),
            None => None,
        }
    }


    fn roll_update_delay_ts(&mut self, peer: &Peer, new_delay: u64) {
        match self.neighbour_list.get_mut(peer) {
            Some(stat) => {
                (*stat).delay_ts = (stat.delay_ts * (DELAY_AVERAGE_WD as u64 - 1)
                    + new_delay) / DELAY_AVERAGE_WD as u64;
            }
            _ => {}
        }
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

    fn set_src_inv_desc(&mut self, tr: &Peer, query: &Peer, _: u64) -> Option<()> {
        todo!()
    }

    fn set_relay_inv_desc(&mut self, tr: &Peer, query: &Peer, _: u64) -> Option<()> {
        todo!()
    }

    fn set_delay_ts(&mut self, query: &Peer, _: u64) -> Option<()> {
        todo!()
    }
}