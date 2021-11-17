use std::collections::HashMap;

pub trait StoreService {

    fn new() -> Self;

    fn pending(&mut self, round: u32, payload: &[u8]) -> bool;

    fn commit(&mut self, round: u32) -> Option<&[u8]>;

    fn get_pending(&self, round: u32) -> Option<&[u8]>;

}


pub struct Store {
    req_by_round: HashMap::<u32, (Vec<u8>, bool)>,
}


impl StoreService for Store {
    fn new() -> Self {
        Self {
            req_by_round: HashMap::new()
        }
    }

    fn pending(&mut self, round: u32, payload: &[u8]) -> bool {
        if let Some((req, committed)) = self.req_by_round.get_mut(&round) {
            if *committed {
                false
            }
            else {
                *req = payload.to_vec();
                true
            }
        }
        else {
            self.req_by_round.insert(round, (payload.to_vec(), false));
            false
        }
    }

    fn commit(&mut self, round: u32) -> Option<&[u8]> {
        if let Some((req, committed)) = self.req_by_round.get_mut(&round) {
            if *committed {
                None
            }
            else {
                *committed = true;
                Some(req)
            }
        }
        else {
            None
        }
    }

    fn get_pending(&self, round: u32) -> Option<&[u8]> {
        if let Some((req, _)) = self.req_by_round.get(&round) {
            Some(req)
        }
        else {
            None
        }
    }
}