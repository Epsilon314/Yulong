use std::collections::{HashMap, hash_map};
use std::hash::Hash;

pub struct BidirctHashmap<K, V> 
    where K: Hash + PartialEq + Eq + Clone,
          V: Hash + PartialEq + Eq + Clone,
{
    k_v: HashMap<K, V>,
    v_k: HashMap<V, K>,
}

impl<K, V> BidirctHashmap<K, V> 
    where K: Hash + PartialEq + Eq + Clone,
          V: Hash + PartialEq + Eq + Clone,
{

    pub fn new() -> Self {
        Self {
            k_v: HashMap::<K, V>::new(),
            v_k: HashMap::<V, K>::new(),
        }
    }

    pub fn contains_key(&self, k: &K) -> bool {
        self.k_v.contains_key(k)
    }

    pub fn contains_value(&self, v: &V) -> bool {
        self.v_k.contains_key(v)
    }

    pub fn get_by_key(&self, k: &K) -> Option<&V> {
        self.k_v.get(k)
    }

    pub fn get_by_value(&self, v: &V) -> Option<&K> {
        self.v_k.get(v)
    }

    pub fn insert(&mut self, k: K, v: V) {
        self.k_v.insert(k.clone(), v.clone());
        self.v_k.insert(v, k);
    }

    pub fn iter(&self) -> hash_map::Iter::<'_, K, V> {
        self.k_v.iter()
    }

}