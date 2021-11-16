use yulong_network::identity::Peer;
use std::collections::{HashMap, HashSet};

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

    fn count_pos(&self) -> u32;

    fn count_neg(&self) -> u32;

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

    fn count_pos(&self) -> u32 {
        self.vote_pos.len() as u32
    }

    fn count_neg(&self) -> u32 {
        self.vote_neg.len() as u32
    }

}


pub struct VoteBoxes {
    voter_n: usize,
    quorum_size: usize,

    boxes_set: HashMap<u32, VoteBox>
}


impl VoteBoxes {

    pub fn new(voter: usize, quorum: usize) -> Self {
        Self {
            voter_n: voter,
            quorum_size: quorum,
            boxes_set: HashMap::new()
        }
    }


    pub fn vote(&mut self, voter: &Peer, pos: bool, round: u32) {
        if let Some(handle) = self.boxes_set.get_mut(&round) {
            handle.vote(voter, pos);
        }
        else {
            let mut new_votebox = VoteBox::new(self.voter_n, self.quorum_size);
            new_votebox.vote(voter, pos);
            self.boxes_set.insert(round, new_votebox);
        }
    }


    pub fn reset(&mut self, round: u32) {
        if let Some(handle) = self.boxes_set.get_mut(&round) {
            handle.reset();
        }
    }


    pub fn result(&self, round: u32) -> VoteResult {
        if let Some(handle) = self.boxes_set.get(&round) {
            handle.result()
        }
        else {
            // zero vote
            VoteResult::PENDING
        }
    }

    pub fn count_pos(&self, round: u32) -> u32 {
        if let Some(handle) = self.boxes_set.get(&round) {
            handle.count_pos()
        }
        else {
            0
        }
    }

    pub fn count_neg(&self, round: u32) -> u32 {
        if let Some(handle) = self.boxes_set.get(&round) {
            handle.count_neg()
        }
        else {
            0
        }
    }

}