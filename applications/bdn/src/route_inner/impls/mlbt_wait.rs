use std::collections::HashMap;

use log::warn;
use yulong_network::identity::Peer;
use yulong::utils::CasualTimer;

#[derive(Clone)]
pub enum WaitStateData {
    JoinWait((Peer, Peer, u64)),   // src, waitfor, require msg id
    JoinPre((Peer, Peer, u64)),   // src, subscriber, ack msg id

    MergeWait((Peer, Peer, u64)), // src, waitfor, ack msg id
    MergePre((Peer, Peer, u64)), // src, requirer, ack msg id
    // src, merge target, accept request msg id, require msg id
    MergeCheck((Peer, Peer, u64)),
    
    GrantWait((Peer, Peer, u64)), // src, receiver, request msg id
    GrantJoin((Peer, Peer, u64)), // src, recv, join msg id
    GrantRecv((Peer, Peer)), // src, expecting 
    GrantTotal((Peer, Peer)), // src, grantee

    RetractWait((Peer, Peer, u64)), // src, recv, request msg id
    RetractJoin((Peer, Peer, u64)), // src, recv, request msg id
    RetractRecv((Peer, Peer, u64)), // src, expecting, request msg id
    RetractTotal((Peer, Peer)), // src, grantee
}


// only know type, do not known associated meta-data
pub enum WaitStateType {
    JoinWait,
    JoinPre,

    MergeWait,
    MergePre,
    MergeCheck,
    
    GrantWait,
    GrantJoin,
    GrantRecv,
    GrantTotal,
    
    RetractWait,
    RetractJoin,
    RetractRecv,
    RetractTotal,
}

// todo: initial from config 
// note for reconstruct:
// MERGE_PRE_TO should exceed MERGE_CHECK_TO + MERGE_WAIT_TO
static JOIN_WAIT_TO: u128 = 2000; // ms
static JOIN_PRE_TO: u128 = 2000; // ms
static MERGE_WAIT_TO: u128 = 2000; // ms
static MERGE_PRE_TO: u128 = 4000; // ms
static MERGE_CHECK_TO: u128 = 2000; // ms
static GRANT_WAIT_TO: u128 = 2000; // ms
static GRANT_JOIN_TO: u128 = 2000; // ms
static RETRACT_WAIT_TO: u128 = 2000; // ms
static RETRACT_JOIN_TO: u128 = 2000; // ms
static GRANT_RECV_TO: u128 = 2000; // ms
static GRANT_TOTAL_TO: u128 = 2000; // ms
static RETRACT_RECV_TO: u128 = 2000; // ms
static RETRACT_TOTAL_TO: u128 = 2000; // ms

trait TimedStatesSingle {

    // get associated data of a type if any
    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // get the data if it is timeout, get None elsewise
    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // set state data and start the timer now
    fn set(&mut self, state: WaitStateData);

    // clear the timed state by type
    fn clear(&mut self, state_type: WaitStateType);

    fn check_id(&self, id: u64) -> Option<WaitStateData>;

}


// TimedStatesSingle expect that indexed by root Peer
// used as the interface
pub trait TimedStates {

    // get associated data of a type if any
    fn get(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData>;

    // get the data if it is timeout, get None elsewise
    fn check(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData>;

    // set state data and start the timer now
    fn set(&mut self, peer: &Peer, state: WaitStateData);

    // clear the timed state by type
    fn clear(&mut self, peer: &Peer, state_type: WaitStateType);

    fn get_by_id(&self, id: u64) -> Option<WaitStateData>;

}


#[derive(Clone)]
struct WaitState {
    wait_timer: CasualTimer,
    inner: WaitStateData,
}


impl WaitState {

    // create a timed state and start the timer, the state is considered stale after timeout
    // todo use macros to be generic over types ?
    fn new(kind: WaitStateData) -> Self {
        let mut ret: Self;

        match kind {
            WaitStateData::JoinWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_WAIT_TO),
                    inner: WaitStateData::JoinWait(data),
                };
            }

            WaitStateData::JoinPre(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_PRE_TO),
                    inner: WaitStateData::JoinPre(data),
                };
            },
            
            WaitStateData::MergeWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_WAIT_TO),
                    inner: WaitStateData::MergeWait(data),
                };
            },
            
            WaitStateData::MergePre(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_PRE_TO),
                    inner: WaitStateData::MergePre(data),
                };
            },
            
            WaitStateData::MergeCheck(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_CHECK_TO),
                    inner: WaitStateData::MergeCheck(data),
                };
            },

            WaitStateData::GrantWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(GRANT_WAIT_TO),
                    inner: WaitStateData::GrantWait(data),
                };
            },

            WaitStateData::GrantJoin(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(GRANT_JOIN_TO),
                    inner: WaitStateData::GrantJoin(data),
                };
            },

            WaitStateData::RetractWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(RETRACT_WAIT_TO),
                    inner: WaitStateData::RetractWait(data),
                };
            },
            
            WaitStateData::RetractJoin(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(RETRACT_JOIN_TO),
                    inner: WaitStateData::RetractJoin(data),
                };
            },
            
            WaitStateData::GrantRecv(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(GRANT_RECV_TO),
                    inner: WaitStateData::GrantRecv(data),
                };
            },
            
            WaitStateData::GrantTotal(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(GRANT_TOTAL_TO),
                    inner: WaitStateData::GrantTotal(data),
                };
            },
            
            WaitStateData::RetractRecv(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(RETRACT_RECV_TO),
                    inner: WaitStateData::RetractRecv(data),
                };
            },
            
            WaitStateData::RetractTotal(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(RETRACT_TOTAL_TO),
                    inner: WaitStateData::RetractTotal(data),
                };
            },
            
        }

        ret.wait_timer.set_now();
        ret
    }

}

struct WaitStats {
    join_wait: Option<WaitState>,
    join_pre: Option<WaitState>,

    merge_wait: Option<WaitState>,
    merge_pre: Option<WaitState>,
    merge_check: Option<WaitState>,

    grant_wait: Option<WaitState>,
    grant_join: Option<WaitState>,
    grant_recv: Option<WaitState>,
    grant_total: Option<WaitState>,

    retract_wait: Option<WaitState>,
    retract_join: Option<WaitState>,
    retract_recv: Option<WaitState>,
    retract_total: Option<WaitState>,

}


impl WaitStats {
    pub fn new() -> Self {
        Self {
            join_wait: None,
            join_pre: None,

            merge_wait: None,
            merge_pre: None,
            merge_check: None,

            grant_wait: None,
            grant_join: None,
            grant_recv: None,
            grant_total: None,
            
            retract_wait: None,
            retract_join: None,
            retract_recv: None,
            retract_total: None,

        }
    }
}


impl TimedStatesSingle for WaitStats {

    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JoinWait => {
                match self.join_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::JoinPre => {
                match self.join_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergeWait => {
                match self.merge_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergePre => {
                match self.merge_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergeCheck => {
                match self.merge_check.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::GrantWait =>  {
                match self.grant_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::GrantJoin => {
                match self.grant_join.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::RetractWait => {
                match self.retract_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::RetractJoin => {
                match self.retract_join.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::GrantRecv => {
                match self.grant_recv.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::GrantTotal => {
                match self.grant_total.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::RetractRecv => {
                match self.retract_recv.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
            WaitStateType::RetractTotal => {
                match self.retract_total.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            
        }
    }


    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JoinWait => {
                match self.join_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::JoinPre => {
                match self.join_pre.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergeWait => {
                match self.merge_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergePre => {
                match self.merge_pre.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergeCheck => {
                match self.merge_check.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::GrantWait => {
                match self.grant_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::GrantJoin => {
                match self.grant_join.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::RetractWait => {
                match self.retract_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::RetractJoin => {
                match self.retract_join.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::GrantRecv => {
                match self.grant_recv.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::GrantTotal => {
                match self.grant_total.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::RetractRecv => {
                match self.retract_recv.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            
            WaitStateType::RetractTotal => {
                match self.retract_total.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
        
        }
    }


    fn set(&mut self, state: WaitStateData) {
        match state {
            WaitStateData::JoinWait(_) => {
                self.join_wait = Some(WaitState::new(state))
            }

            WaitStateData::JoinPre(_) => {
                self.join_pre = Some(WaitState::new(state))
            }

            WaitStateData::MergeWait(_) => {
                self.merge_wait = Some(WaitState::new(state))
            }

            WaitStateData::MergePre(_) => {
                self.merge_pre = Some(WaitState::new(state))
            }

            WaitStateData::MergeCheck(_) => {
                self.merge_check = Some(WaitState::new(state))
            }

            WaitStateData::GrantWait(_) => {
                self.grant_wait = Some(WaitState::new(state))
            }

            WaitStateData::GrantJoin(_) => {
                self.grant_join = Some(WaitState::new(state))
            }
            
            WaitStateData::RetractWait(_) => {
                self.retract_wait = Some(WaitState::new(state))
            }
            
            WaitStateData::RetractJoin(_) => {
                self.retract_join = Some(WaitState::new(state))
            }
            
            WaitStateData::GrantRecv(_) => {
                self.grant_recv = Some(WaitState::new(state))
            }
            
            WaitStateData::GrantTotal(_) => {
                self.grant_total = Some(WaitState::new(state))
            }
            
            WaitStateData::RetractRecv(_) => {
                self.retract_recv = Some(WaitState::new(state))
            }
            
            WaitStateData::RetractTotal(_) => {
                self.retract_total = Some(WaitState::new(state))
            }
            
            
        }
    }


    fn clear(&mut self, state_type: WaitStateType) {
        match state_type {
            WaitStateType::JoinWait => {
                self.join_wait = None
            }

            WaitStateType::JoinPre => {
                self.join_pre = None
            }

            WaitStateType::MergeWait => {
                self.merge_wait = None
            }

            WaitStateType::MergePre => {
                self.merge_pre = None
            }

            WaitStateType::MergeCheck => {
                self.merge_check = None
            }

            WaitStateType::GrantWait => {
                self.grant_wait = None
            }

            WaitStateType::GrantJoin => {
                self.grant_join = None
            }
            
            WaitStateType::RetractWait => {
                self.retract_wait = None
            }
            
            WaitStateType::RetractJoin => {
                self.retract_join = None
            }
            
            WaitStateType::GrantRecv => {
                self.grant_recv = None
            }
            
            WaitStateType::GrantTotal => {
                self.grant_total = None
            }
            
            WaitStateType::RetractRecv => {
                self.retract_recv = None
            }
            
            WaitStateType::RetractTotal => {
                self.retract_total = None
            }
            
        }
    }

    fn check_id(&self, id: u64) -> Option<WaitStateData> {

        if let Some(join_wait_data) = &self.join_wait {
            match &join_wait_data.inner {
                WaitStateData::JoinWait((_, _, sid)) => {
                    if *sid == id {
                        return Some(join_wait_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(join_pre_data) = &self.join_pre {
            match &join_pre_data.inner {
                WaitStateData::JoinPre((_, _, sid)) => {
                    if *sid == id {
                        return Some(join_pre_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_wait_data) = &self.merge_wait {
            match &merge_wait_data.inner {
                WaitStateData::MergeWait((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_wait_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_pre_data) = &self.merge_pre {
            match &merge_pre_data.inner {
                WaitStateData::MergePre((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_pre_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_check_data) = &self.merge_check {
            match &merge_check_data.inner {
                WaitStateData::MergeCheck((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_check_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        None
    }

}

pub struct WaitList {
    inner_list: HashMap<Peer, WaitStats>,
}


impl WaitList {

    pub fn new() -> Self {
        Self {
            inner_list: HashMap::new(),
        } 
    }


    // todo: update me
    pub fn is_waiting(&self, root: &Peer) -> bool {
        self.get(root, WaitStateType::JoinWait).is_some() ||
        self.get(root, WaitStateType::JoinPre).is_some() ||
        self.get(root, WaitStateType::MergeWait).is_some() ||
        self.get(root, WaitStateType::MergeWait).is_some() ||
        self.get(root, WaitStateType::MergeCheck).is_some() ||
        self.get(root, WaitStateType::GrantJoin).is_some() ||
        self.get(root, WaitStateType::GrantRecv).is_some() ||
        self.get(root, WaitStateType::GrantRecv).is_some() ||
        self.get(root, WaitStateType::GrantTotal).is_some() ||
        self.get(root, WaitStateType::RetractJoin).is_some() ||
        self.get(root, WaitStateType::RetractWait).is_some() ||
        self.get(root, WaitStateType::RetractRecv).is_some() ||
        self.get(root, WaitStateType::RetractTotal).is_some() 

    }

}


impl TimedStates for WaitList {
    
    fn get(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData> {
        match self.inner_list.get(peer) {
            Some(states) => {
                states.get(state_type)
            }
            None => None
        }   
    }


    fn check(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData> {
        match self.inner_list.get(peer) {
            Some(states) => {
                states.check(state_type)
            }
            None => None
        }
    }


    fn set(&mut self, peer: &Peer, state: WaitStateData) {
        match self.inner_list.get_mut(peer) {
            Some(states) => {
                states.set(state)
            }
            None => {
                // todo log it or throw errors up?
                warn!("")
            }
        }
    }

    fn clear(&mut self, peer: &Peer, state_type: WaitStateType) {
        match self.inner_list.get_mut(peer) {
            Some(states) => {
                states.clear(state_type)
            }
            None => {
                // todo log it or throw errors up?
                warn!("")
            }
        }
    }

    fn get_by_id(&self, id: u64) -> Option<WaitStateData> {
        for data in self.inner_list.values() {
            match data.check_id(id) {
                Some(data) => return Some(data),
                None => {}
            }
        }
        // no matching data
        None
    }

}