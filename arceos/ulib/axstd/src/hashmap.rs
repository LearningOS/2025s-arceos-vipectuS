extern crate alloc;

use alloc::vec::Vec;
use arceos_api::modules::axhal;
use core::hash::{Hash, Hasher};

pub struct SimpleHasher(u128);

impl Hasher for SimpleHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= (b as u128).wrapping_add(0x9e3779b97f4a7c15).rotate_left(11);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(&self) -> u64 {
        (self.0 as u64) ^ ((self.0 >> 64) as u64)
    }
}

pub struct HashMap<K, V> {
    items: Vec<Vec<(K, V)>>,
    hash_salt: u128,
    size: usize,
}

impl<K: Eq + Hash, V> HashMap<K, V> {
    pub fn new() -> Self {
        Self::with_capacity(16)
    }

    pub fn with_capacity(cap: usize) -> Self {
        let mut items = Vec::with_capacity(cap);
        for _ in 0..cap {
            items.push(Vec::new());
        }

        HashMap {
            items,
            hash_salt: axhal::misc::random(),
            size: 0,
        }
    }

    fn hash(&self, key: &K) -> usize {
        let mut hasher = SimpleHasher(self.hash_salt);
        key.hash(&mut hasher);
        (hasher.finish() as usize) % self.items.len()
    }

    pub fn insert(&mut self, key: K, value: V) {
        if self.size * 4 >= self.items.len() * 3 {
            self.resize();
        }

        let idx = self.hash(&key);
        for entry in &mut self.items[idx] {
            if entry.0 == key {
                entry.1 = value;
                return;
            }
        }

        self.items[idx].push((key, value));
        self.size += 1;
    }

    fn resize(&mut self) {
        let new_cap = self.items.len() * 2;
        let mut new_items = Vec::with_capacity(new_cap);
        for _ in 0..new_cap {
            new_items.push(Vec::new());
        }

        for item in self.items.drain(..) {
            for (k, v) in item {
                let mut hasher = SimpleHasher(self.hash_salt);
                k.hash(&mut hasher);
                let idx = (hasher.finish() as usize) % new_cap;
                new_items[idx].push((k, v));
            }
        }

        self.items = new_items;
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.items
            .iter()
            .flat_map(|item| item.iter().map(|(k, v)| (k, v)))
    }
}
