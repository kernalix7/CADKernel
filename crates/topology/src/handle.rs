use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

/// A generational handle that serves as a type-safe, dangling-proof reference
/// to an entity in an [`EntityStore`](super::store::EntityStore).
#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct Handle<T> {
    pub(crate) index: u32,
    pub(crate) generation: u64,
    #[serde(skip)]
    pub(crate) _marker: PhantomData<T>,
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> std::hash::Hash for Handle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}

impl<T> Handle<T> {
    /// Returns the slot index inside the arena.
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Returns the generation counter used for dangling-handle detection.
    #[inline]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Reconstructs a handle from raw index and generation values.
    ///
    /// # Safety (logical)
    ///
    /// The caller must ensure the pair corresponds to an entity that is still
    /// alive in its [`EntityStore`](super::store::EntityStore). Passing stale
    /// or fabricated values will not cause UB but will make subsequent lookups
    /// return `None`.
    #[inline]
    pub fn from_raw_parts(index: u32, generation: u64) -> Self {
        Self {
            index,
            generation,
            _marker: PhantomData,
        }
    }
}
