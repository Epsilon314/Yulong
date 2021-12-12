use std::collections::HashMap;
use std::ops::Deref;

use rand::Rng;

use yulong::utils::CasualTimer;
use crate::config::HEARTBEAT_INV;
use crate::config::ELECTION_INV_LOW;
use crate::config::ELECTION_INV_HIGH;

#[derive(Clone)]
pub struct WaitState {
    wait_timer: CasualTimer,
    inner: WaitStateData,
}


impl Deref for WaitState {
    
    type Target = WaitStateData;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}


impl WaitState {
    fn is_timeout(&self) -> bool {
        self.wait_timer.is_timeout()
    }
}


#[derive(Clone)]
pub enum WaitStateData {
    ApplyEntries(u64, u64),     // next_idx, len of entries
}

pub struct RaftTimer {
    heartbeat_timer: CasualTimer,
    election_timer: Option<CasualTimer>,

    replys: HashMap<u32, WaitState>
}

impl RaftTimer {

    pub fn new() -> Self {
        Self {
            heartbeat_timer: CasualTimer::new(HEARTBEAT_INV),
            election_timer: None,
            replys: HashMap::new()
        }
    }


    pub fn start_heartbeat(&mut self) {self.heartbeat_timer.set_now()}

    pub fn is_heartbeat_timeout(&mut self) -> bool {self.heartbeat_timer.is_timeout()}


    pub fn start_election_timer(&mut self) {

        let mut rng = rand::thread_rng();
        let randomized_timeout = rng.gen_range(ELECTION_INV_LOW..ELECTION_INV_HIGH);

        let mut election_timer = CasualTimer::new(randomized_timeout);
        election_timer.set_now();

        self.election_timer = Some(election_timer);
    }


    pub fn is_election_timeout(&mut self) -> bool {
        if let Some(election_timer) = &self.election_timer {
            election_timer.is_timeout()
        }
        else {
            false
        }
    }


    pub fn get_wait_data_by_id(&self, id: u32) -> Option<WaitState> {
        self.replys.get(&id).map(|s| s.clone())
    }


    pub fn insert_wait_data(&mut self, id: u32, data: &WaitState) -> Option<WaitState> {
        self.replys.insert(id, data.to_owned())
    }

}