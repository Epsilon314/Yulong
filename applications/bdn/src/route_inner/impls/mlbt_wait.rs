use std::collections::HashMap;

use log::warn;
use yulong_network::identity::Peer;
use yulong::utils::CasualTimer;


#[derive(Clone)]
pub enum WaitStateData {
    JOIN_WAIT((Peer, Peer, u64)),   // src, waitfor, require msg id
    JOIN_PRE((Peer, Peer, u64)),   // src, subscriber, ack msg id
    MERGE_WAIT((Peer, Peer, u64)), // src, waitfor, ack msg id
    MERGE_PRE((Peer, Peer, u64)), // src, requirer, ack msg id
    MERGE_CHECK((Peer, u64)), // src, require msg id
}


// only know type, do not known associated meta-data
pub enum WaitStateType {
    JOIN_WAIT,
    JOIN_PRE,
    MERGE_WAIT,
    MERGE_PRE,
    MERGE_CHECK,
}

static JOIN_WAIT_TO: u128 = 2000; // ms
static JOIN_PRE_TO: u128 = 2000; // ms
static MERGE_WAIT_TO: u128 = 2000; // ms
static MERGE_PRE_TO: u128 = 2000; // ms
static MERGE_CHECK_TO: u128 = 2000; // ms

trait TimedStatesSingle {

    // get associated data of a type if any
    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // get the data if it is timeout, get None elsewise
    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // set state data and start the timer now
    fn set(&mut self, state: WaitStateData);

    // clear the timed state by type
    fn clear(&mut self, state_type: WaitStateType);

    fn check_id(&self, id: u64) -> bool;

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
            WaitStateData::JOIN_WAIT(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_WAIT_TO),
                    inner: WaitStateData::JOIN_WAIT(data),
                };
            }

            WaitStateData::JOIN_PRE(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_PRE_TO),
                    inner: WaitStateData::JOIN_PRE(data),
                };
            },
            
            WaitStateData::MERGE_WAIT(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_WAIT_TO),
                    inner: WaitStateData::MERGE_WAIT(data),
                };
            },
            
            WaitStateData::MERGE_PRE(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_PRE_TO),
                    inner: WaitStateData::MERGE_PRE(data),
                };
            },
            
            WaitStateData::MERGE_CHECK(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_CHECK_TO),
                    inner: WaitStateData::MERGE_CHECK(data),
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
}


impl WaitStats {
    pub fn new() -> Self {
        Self {
            join_wait: None,
            join_pre: None,

            merge_wait: None,
            merge_pre: None,
            merge_check: None,
        }
    }
}


impl TimedStatesSingle for WaitStats {

    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JOIN_WAIT => {
                match self.join_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::JOIN_PRE => {
                match self.join_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MERGE_WAIT => {
                match self.merge_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MERGE_PRE => {
                match self.merge_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MERGE_CHECK => {
                match self.merge_check.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
        }
    }


    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JOIN_WAIT => {
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

            WaitStateType::JOIN_PRE => {
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

            WaitStateType::MERGE_WAIT => {
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

            WaitStateType::MERGE_PRE => {
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

            WaitStateType::MERGE_CHECK => {
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
        }
    }


    fn set(&mut self, state: WaitStateData) {
        match state {
            WaitStateData::JOIN_WAIT(_) => {
                self.join_wait = Some(WaitState::new(state))
            }

            WaitStateData::JOIN_PRE(_) => {
                self.join_pre = Some(WaitState::new(state))
            }

            WaitStateData::MERGE_WAIT(_) => {
                self.merge_wait = Some(WaitState::new(state))
            }

            WaitStateData::MERGE_PRE(_) => {
                self.merge_pre = Some(WaitState::new(state))
            }

            WaitStateData::MERGE_CHECK(_) => {
                self.merge_check = Some(WaitState::new(state))
            }
        }
    }


    fn clear(&mut self, state_type: WaitStateType) {
        match state_type {
            WaitStateType::JOIN_WAIT => {
                self.join_wait = None
            }

            WaitStateType::JOIN_PRE => {
                self.join_pre = None
            }

            WaitStateType::MERGE_WAIT => {
                self.merge_wait = None
            }

            WaitStateType::MERGE_PRE => {
                self.merge_pre = None
            }

            WaitStateType::MERGE_CHECK => {
                self.merge_check = None
            }
        }
    }

    fn check_id(&self, id: u64) -> bool {
        todo!()
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


    pub fn is_waiting(&self, root: &Peer) -> bool {
        self.get(root, WaitStateType::JOIN_WAIT).is_some() ||
        self.get(root, WaitStateType::JOIN_PRE).is_some() ||
        self.get(root, WaitStateType::MERGE_WAIT).is_some() ||
        self.get(root, WaitStateType::MERGE_WAIT).is_some() ||
        self.get(root, WaitStateType::MERGE_CHECK).is_some()
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
        todo!()
    }

}