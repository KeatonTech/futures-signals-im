use core::hash::Hash;
use im::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapDiff<K> {
    Replace {},

    Insert {
        key: K,
    },

    Remove {
        key: K,
    },

    Clear {},
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashMapEvent<K, V> where K: Clone + Eq + Hash, V: Clone {
    pub snapshot: HashMap<K, V>,
    pub diff: MapDiff<K>
}