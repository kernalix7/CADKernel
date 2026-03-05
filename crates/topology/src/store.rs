use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use crate::handle::Handle;

#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
struct Entry<T> {
    value: Option<T>,
    generation: u32,
}

/// An arena-style store that allocates entities and returns [`Handle`]s.
///
/// Supports O(1) insert, remove, and lookup. Generations prevent use-after-free
/// via stale handles.
#[derive(Clone, Serialize, Deserialize)]
#[serde(bound(serialize = "T: Serialize", deserialize = "T: Deserialize<'de>"))]
pub struct EntityStore<T> {
    entries: Vec<Entry<T>>,
    free_list: Vec<u32>,
    alive_count: usize,
}

impl<T> EntityStore<T> {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            free_list: Vec::new(),
            alive_count: 0,
        }
    }

    /// Inserts a value and returns a handle to it.
    pub fn insert(&mut self, value: T) -> Handle<T> {
        self.alive_count += 1;
        if let Some(index) = self.free_list.pop() {
            let entry = &mut self.entries[index as usize];
            entry.value = Some(value);
            Handle {
                index,
                generation: entry.generation,
                _marker: PhantomData,
            }
        } else {
            let index = self.entries.len() as u32;
            self.entries.push(Entry {
                value: Some(value),
                generation: 0,
            });
            Handle {
                index,
                generation: 0,
                _marker: PhantomData,
            }
        }
    }

    /// Removes the entity at `handle`, returning it if it was still alive.
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        if entry.generation != handle.generation || entry.value.is_none() {
            return None;
        }
        let value = entry.value.take();
        entry.generation += 1;
        self.free_list.push(handle.index);
        self.alive_count -= 1;
        value
    }

    /// Returns a reference to the entity if the handle is still valid.
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        let entry = self.entries.get(handle.index as usize)?;
        if entry.generation != handle.generation {
            return None;
        }
        entry.value.as_ref()
    }

    /// Returns a mutable reference to the entity if the handle is still valid.
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        let entry = self.entries.get_mut(handle.index as usize)?;
        if entry.generation != handle.generation {
            return None;
        }
        entry.value.as_mut()
    }

    /// Returns `true` if the handle still points to a live entity.
    pub fn is_alive(&self, handle: Handle<T>) -> bool {
        self.get(handle).is_some()
    }

    /// Number of live entities (O(1)).
    #[inline]
    pub fn len(&self) -> usize {
        self.alive_count
    }

    /// Returns `true` if no entities are stored.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.alive_count == 0
    }

    /// Iterates over all live (handle, &value) pairs.
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.entries.iter().enumerate().filter_map(|(i, entry)| {
            entry.value.as_ref().map(|v| {
                (
                    Handle {
                        index: i as u32,
                        generation: entry.generation,
                        _marker: PhantomData,
                    },
                    v,
                )
            })
        })
    }

    /// Iterates over all live (handle, &mut value) pairs.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Handle<T>, &mut T)> {
        self.entries
            .iter_mut()
            .enumerate()
            .filter_map(|(i, entry)| {
                let generation = entry.generation;
                entry.value.as_mut().map(|v| {
                    (
                        Handle {
                            index: i as u32,
                            generation,
                            _marker: PhantomData,
                        },
                        v,
                    )
                })
            })
    }
}

impl<T> Default for EntityStore<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut store = EntityStore::new();
        let h = store.insert(42);
        assert_eq!(store.get(h), Some(&42));
    }

    #[test]
    fn test_remove_invalidates_handle() {
        let mut store = EntityStore::new();
        let h = store.insert(10);
        store.remove(h);
        assert!(store.get(h).is_none());
    }

    #[test]
    fn test_generation_prevents_stale_access() {
        let mut store = EntityStore::new();
        let h1 = store.insert(100);
        store.remove(h1);
        let h2 = store.insert(200);
        assert!(store.get(h1).is_none());
        assert_eq!(store.get(h2), Some(&200));
        assert_ne!(h1.generation, h2.generation);
    }

    #[test]
    fn test_len() {
        let mut store = EntityStore::new();
        let h1 = store.insert(1);
        let _h2 = store.insert(2);
        assert_eq!(store.len(), 2);
        store.remove(h1);
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn test_iter() {
        let mut store = EntityStore::new();
        store.insert(10);
        store.insert(20);
        store.insert(30);
        let values: Vec<_> = store.iter().map(|(_, v)| *v).collect();
        assert_eq!(values, vec![10, 20, 30]);
    }
}
