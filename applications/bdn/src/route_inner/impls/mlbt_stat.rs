

// query and update mlbt stat
// mlbt stat:
// src_inv:
// relay_inv:
pub trait MlbtStatMaintainer {

    fn src_inv(&self) -> f32;
    fn relay_inv(&self) -> f32;

}


// directly modify mlbt stat for test and debug simplicity
pub trait MlbtStatDebug {
    fn set_src_inv(&mut self, _: f32);
    fn set_relay_inv(&mut self, _: f32);
}


pub struct MlbtStat {
    src_inv: f32,
    relay_inv: f32,
}


impl MlbtStat {

    pub fn new() -> Self {
        Self {
            // todo
            src_inv: 0.0,
            relay_inv: 0.0,
        }
    }

}


impl MlbtStatMaintainer for MlbtStat {
    fn src_inv(&self) -> f32 {
        todo!()
    }

    fn relay_inv(&self) -> f32 {
        todo!()
    }
}


impl MlbtStatDebug for MlbtStat {
    fn set_src_inv(&mut self, _: f32) {
        todo!()
    }

    fn set_relay_inv(&mut self, _: f32) {
        todo!()
    }
}