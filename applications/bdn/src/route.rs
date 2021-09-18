use yulong_network::{identity::Peer, transport::Transport};
use std::{collections::HashMap, marker::PhantomData};

pub struct Route<T: Transport> {
    transport_type: PhantomData<T>,
}

impl<T: Transport> Route<T> {

    pub fn new() -> Self {
        Self {
            transport_type: PhantomData::<T>,
        }
    }

    pub fn next_hop(src: &Peer, dst: &Peer) -> Peer {
        unimplemented!()
    }

    pub fn next_hops(src: &Peer, from: &Peer) -> Vec<Peer> {
        unimplemented!()
    }
}


#[cfg(test)]
mod test {
    use super::Route;
}