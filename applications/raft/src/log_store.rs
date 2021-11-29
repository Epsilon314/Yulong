pub(crate) trait LogService {

    // leader shall accept log entries from clients and replicate
    // them across the cluster
    fn client_new_entry(payload: LogEntry);

    fn commit(&mut self, idx: usize);
}


pub(crate) struct LogEntry {
    term: u64,
    command: Vec<u8>,
}


pub struct ReplicatedLog {
    
}


impl LogService for ReplicatedLog {

    fn client_new_entry(payload: LogEntry) {
        todo!()
    }

    fn commit(&mut self, _: usize) { todo!() }
}


impl ReplicatedLog {

}