mod message;
mod pbft_message {
    include!(concat!(env!("OUT_DIR"), "/pbft.rs"));
}
mod quorum;
mod test;

pub mod pbft;
