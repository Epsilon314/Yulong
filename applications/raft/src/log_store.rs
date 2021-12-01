pub(crate) trait LogService {

    // leader shall accept log entries from clients and replicate
    // them across the cluster
    fn client_new_entry(payload: LogEntry);

    fn commit(&mut self, idx: u64);

    fn last(&self) -> (u64, LogEntry);
}


pub(crate) struct LogEntry {
    term: u64,
    command: Vec<u8>,
}

impl LogEntry {
    /// Get a reference to the log entry's term.
    pub(crate) fn term(&self) -> u64 {
        self.term
    }
}


pub struct ReplicatedLog {
    
}


impl LogService for ReplicatedLog {

    fn client_new_entry(payload: LogEntry) {
        todo!()
    }

    fn commit(&mut self, _: u64) { todo!() }

    fn last(&self) -> (u64, LogEntry) {
        todo!()
    }
}


impl ReplicatedLog {

}