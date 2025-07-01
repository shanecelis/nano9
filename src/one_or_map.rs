use bevy::{
    utils::{HashMap, hashbrown::Equivalent},
    prelude::*,
};
use std::hash::Hash;


/// One or map
///
/// This enum is designed to primarily hold one value without any heap
/// allocation but may be promoted to a HashMap that is stored in the heap.
#[derive(Debug, Reflect)]
pub enum OneOrMap<K,V> {
    One { key: K, value: V },
    Map(HashMap<K, V>),
}

impl<K,V> OneOrMap<K,V> {
    pub fn new(key: K, value: V) -> Self {
        OneOrMap::One { key, value }
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
        where
        // K: Borrow<Q>,
        K: Hash + Eq,
        Q: Hash + Equivalent<K> + ?Sized,
    {
        match self {
            OneOrMap::One { key: single_key, value } => {
                if key.equivalent(single_key) {
                    Some(value)
                } else {
                    // Create an image an ugrade to a Multiple.
                    None
                }
            }
            OneOrMap::Map(map) => {
                map.get(key)
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V>
    where K: Eq + Hash {
        match self {
            OneOrMap::One { key: single_key, value: single_value } => {
                // if key.equivalent(single_key) {
                if *single_key == key {
                    Some(std::mem::replace(single_value, value))
                } else {
                    let mut map = HashMap::with_capacity(2);
                    map.insert(key, value);
                    let last_value = std::mem::replace(self, OneOrMap::Map(map));
                    match last_value {
                        OneOrMap::One { key: single_key, value: single_value } => {
                            self.insert(single_key, single_value);
                        }
                        _ => unreachable!(),
                    }
                    None
                }
            }
            OneOrMap::Map(map) => map.insert(key, value),
        }
    }
}
