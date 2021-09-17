mod message;
mod route;
pub mod overlay;

mod bdn_message {
    include!(concat!(env!("OUT_DIR"), "/bdn.rs"));
}