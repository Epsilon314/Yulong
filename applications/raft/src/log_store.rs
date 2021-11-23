pub trait LogService {

    // leader shall accept log entries from clients and replicate
    // them across the cluster
    fn client_new_entry(payload: &[u8]);

}


pub struct ReplicatedLog {
    
}