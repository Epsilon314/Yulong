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

    /// Use two hashmap to maintain a bi-direction hashmap
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


    /// Insert into two hashmap
    pub fn insert(&mut self, k: &K, v: &V) {
        self.k_v.insert(k.clone(), v.clone());
        self.v_k.insert(v.clone(), k.clone());
    }


    /// Update the value of given key
    /// Since impl get_mut for BidirctHashmap is non-trivial, 
    /// we use this function as a alternative.
    /// 
    /// Return Some(()) if update successfully and None is the given key 
    /// do not exists.
    pub fn update_by_key(&mut self, k: &K, v: &V) -> Option<()> {
        if let Some(v_handle) = self.k_v.get_mut(k) {
            
            // update k_v
            *v_handle = v.clone();

            // update v_k
            // since (k,v_handle) is found in k_v, (v_handle, k) must be in v_k
            self.v_k.remove(v_handle).unwrap();
            self.v_k.insert(v.clone(), k.clone());

            Some(())
        }

        // do not contain certain key
        else {
            None
        }
    }


    /// Update the value of given value
    /// Counter-part of update_by_key
    pub fn update_by_value(&mut self, v: &V, k: &K) -> Option<()> {
        if let Some(k_handle) = self.v_k.get_mut(v) {
            
            *k_handle = k.clone();

            self.k_v.remove(k_handle).unwrap();
            self.k_v.insert(k.clone(), v.clone());

            Some(())
        }
        else {
            None
        }
    }


    // default iter by key
    pub fn iter(&self) -> hash_map::Iter::<'_, K, V> {
        self.k_v.iter()
    }
   
}