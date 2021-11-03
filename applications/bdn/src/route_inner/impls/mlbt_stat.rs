

// query and update mlbt stat
// mlbt stat:
// src_inv: interval between root start -> finish relaying
// relay_inv: interval between recv -> all desc recv
pub trait MlbtStatMaintainer {

    fn src_inv(&self) -> u64;

    fn relay_inv(&self) -> u64;

    fn merge_thrd(&self) -> u64;

    // src_inv update cb
    // init src_inv update (root node only)

    // relay_inv update cb
    // init relay_inv update

}


// directly modify mlbt stat for test and debug simplicity
pub trait MlbtStatDebug {
    
    fn set_src_inv(&mut self, _: u64);
    
    fn set_relay_inv(&mut self, _: u64);

}


pub struct MlbtStat {
    src_inv: u64,
    relay_inv: u64,
    merge_thrd: u64,
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


impl MlbtStatMaintainer for MlbtStat {
    fn src_inv(&self) -> u64 {
        self.src_inv
    }

    fn relay_inv(&self) -> u64 {
        self.relay_inv
    }

    fn merge_thrd(&self) -> u64 {
        self.merge_thrd
    }
}


impl MlbtStatDebug for MlbtStat {
    fn set_src_inv(&mut self, new_value: u64) {
        self.src_inv = new_value;
    }

    fn set_relay_inv(&mut self, new_value: u64) {
        self.relay_inv = new_value;
    }
}