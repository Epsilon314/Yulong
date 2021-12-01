use rand::Rng;

use yulong::utils::CasualTimer;
use crate::config::HEARTBEAT_INV;
use crate::config::ELECTION_INV_LOW;
use crate::config::ELECTION_INV_HIGH;


pub struct RaftTimer {
    heartbeat_timer: CasualTimer,
    election_timer: Option<CasualTimer>,
}

impl RaftTimer {

    pub fn new() -> Self {
        Self {
            heartbeat_timer: CasualTimer::new(HEARTBEAT_INV),
            election_timer: None,
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

}