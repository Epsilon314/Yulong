use yulong_network::identity::Peer;
use std::collections::HashSet;

pub struct VoteBox {
    voter_n: usize,
    quorum_size: usize,
    vote_pos: HashSet<Peer>,
    vote_neg: HashSet<Peer>
}

#[derive(PartialEq)]
pub enum VoteResult {
    PASS,
    FAIL,
    PENDING
}

pub trait QuorumCollector {

    fn new(voter: usize, quorum: usize) -> Self;

    fn vote(&mut self, voter: &Peer, pos: bool);

    fn reset(&mut self);

    fn result(&self) -> VoteResult;

}


impl QuorumCollector for VoteBox {
    
    fn new(voter: usize, quorum: usize) -> Self {
        Self {
            voter_n: voter,
            quorum_size: quorum,
            vote_pos: HashSet::new(),
            vote_neg: HashSet::new(),
        }
    }

    fn vote(&mut self, voter: &Peer, pos: bool) {
        if pos {
            self.vote_pos.insert(voter.to_owned());
        }
        else {
            self.vote_neg.insert(voter.to_owned());
        }
    }

    fn reset(&mut self) {
        self.vote_pos.clear();
        self.vote_neg.clear();
    }

    fn result(&self) -> VoteResult {
        if self.vote_pos.len() >= self.quorum_size {
            return VoteResult::PASS;
        }
        if self.vote_neg.len() >= self.quorum_size {
            return VoteResult::FAIL;
        }
        VoteResult::PENDING
    }

}