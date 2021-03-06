#![license = "MIT"]
#![deny(missing_docs)]
#![deny(warnings)]

//! A type-based key value store where one value type is allowed for each key.

extern crate alloc;
extern crate "unsafe-any" as uany;

use std::any::Any;
use std::intrinsics::TypeId;
use std::collections::{hashmap, HashMap};

// These traits are faster when we know the type is correct already.
use uany::{UncheckedAnyDowncast, UncheckedAnyMutDowncast, UncheckedBoxAnyDowncast};

/// A map keyed by types.
///
/// Can contain one value of any type for each key type, as defined
/// by the Assoc trait.
pub struct TypeMap {
    data: HashMap<TypeId, Box<Any + 'static>>
}

/// This trait defines the relationship between keys and values in a TypeMap.
///
/// It is implemented for Keys, with a phantom type parameter for values.
pub trait Assoc<Value: 'static>: 'static {}

impl TypeMap {
    /// Create a new, empty TypeMap.
    pub fn new() -> TypeMap {
        TypeMap {
            data: HashMap::new()
        }
    }

    /// Insert a value into the map with a specified key type.
    pub fn insert<K: Assoc<V>, V: 'static>(&mut self, val: V) -> bool {
        self.data.insert(TypeId::of::<K>(), box val as Box<Any>)
    }

    /// Find a value in the map and get a reference to it.
    pub fn find<K: Assoc<V>, V: 'static>(&self) -> Option<&V> {
        self.data.find(&TypeId::of::<K>()).map(|v| unsafe {
            v.downcast_ref_unchecked::<V>()
        })
    }

    /// Find a value in the map and get a mutable reference to it.
    pub fn find_mut<K: Assoc<V>, V: 'static>(&mut self) -> Option<&mut V> {
        self.data.find_mut(&TypeId::of::<K>()).map(|v| unsafe {
            v.downcast_mut_unchecked::<V>()
        })
    }

    /// Check if a key has an associated value stored in the map.
    pub fn contains<K: Assoc<V>, V: 'static>(&self) -> bool {
        self.data.contains_key(&TypeId::of::<K>())
    }

    /// Remove a value from the map.
    ///
    /// Returns `true` if a value was removed.
    pub fn remove<K: Assoc<V>, V: 'static>(&mut self) -> bool {
        self.data.remove(&TypeId::of::<K>())
    }

    /// Get the given key's corresponding entry in the map for in-place manipulation.
    pub fn entry<'a, K: Assoc<V>, V: 'static>(&'a mut self) -> Entry<'a, K, V> {
        match self.data.entry(TypeId::of::<K>()) {
            hashmap::Occupied(e) => Occupied(OccupiedEntry { data: e }),
            hashmap::Vacant(e) => Vacant(VacantEntry { data: e })
        }
    }

    /// Read the underlying HashMap
    pub unsafe fn data(&self) -> &HashMap<TypeId, Box<Any + 'static>> { &self.data }

    /// Get a mutable reference to the underlying HashMap
    pub unsafe fn data_mut(&mut self) -> &mut HashMap<TypeId, Box<Any + 'static>> { &mut self.data }

    /// Get the number of values stored in the map.
    pub fn len(&self) -> uint {
        self.data.len()
    }

    /// Return true if the map contains no values.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Remove all entries from the map.
    pub fn clear(&mut self) {
        self.data.clear()
    }
}

/// A view onto an entry in a TypeMap.
pub enum Entry<'a, K, V> {
    /// A view onto an occupied entry in a TypeMap.
    Occupied(OccupiedEntry<'a, K, V>),
    /// A view onto an unoccupied entry in a TypeMap.
    Vacant(VacantEntry<'a, K, V>)
}

/// A view onto an occupied entry in a TypeMap.
pub struct OccupiedEntry<'a, K, V> {
    data: hashmap::OccupiedEntry<'a, TypeId, Box<Any + 'static>>
}

/// A view onto an unoccupied entry in a TypeMap.
pub struct VacantEntry<'a, K, V> {
    data: hashmap::VacantEntry<'a, TypeId, Box<Any + 'static>>
}

impl<'a, K, V: 'static> OccupiedEntry<'a, K, V> {
    /// Get a reference to the entry's value.
    pub fn get(&self) -> &V {
        unsafe {
            self.data.get().downcast_ref_unchecked::<V>()
        }
    }

    /// Get a mutable reference to the entry's value.
    pub fn get_mut(&mut self) -> &mut V {
        unsafe {
            self.data.get_mut().downcast_mut_unchecked::<V>()
        }
    }

    /// Transform the entry into a mutable reference with the same lifetime as the map.
    pub fn into_mut(self) -> &'a mut V {
        unsafe {
            self.data.into_mut().downcast_mut_unchecked::<V>()
        }
    }

    /// Set the entry's value and return the previous value.
    pub fn set(&mut self, value: V) -> V {
        unsafe {
            *self.data.set(box value as Box<Any + 'static>).downcast_unchecked::<V>()
        }
    }

    /// Move the entry's value out of the map, consuming the entry.
    pub fn take(self) -> V {
        unsafe {
            *self.data.take().downcast_unchecked::<V>()
        }
    }
}

impl<'a, K, V: 'static> VacantEntry<'a, K, V> {
    /// Set the entry's value and return a mutable reference to it.
    pub fn set(self, value: V) -> &'a mut V {
        unsafe {
            self.data.set(box value as Box<Any + 'static>).downcast_mut_unchecked::<V>()
        }
    }
}

#[cfg(test)]
mod test {
    use super::{TypeMap, Assoc, Occupied, Vacant};

    #[deriving(Show, PartialEq)]
    struct Key;

    #[deriving(Show, PartialEq)]
    struct Value;

    impl Assoc<Value> for Key {}

    #[test] fn test_pairing() {
        let mut map = TypeMap::new();
        map.insert::<Key, Value>(Value);
        assert_eq!(*map.find::<Key, Value>().unwrap(), Value);
        assert!(map.contains::<Key, Value>());
    }

    #[test] fn test_remove() {
        let mut map = TypeMap::new();
        map.insert::<Key, Value>(Value);
        assert!(map.contains::<Key, Value>());
        map.remove::<Key, Value>();
        assert!(!map.contains::<Key, Value>());
    }

    #[test] fn test_entry() {
        let mut map = TypeMap::new();
        map.insert::<Key, Value>(Value);
        match map.entry::<Key, Value>() {
            Occupied(e) => {
                assert_eq!(e.get(), &Value);
                assert_eq!(e.take(), Value);
            },
            _ => panic!("Unable to locate inserted item.")
        }
        assert!(!map.contains::<Key, Value>());
        match map.entry::<Key, Value>() {
            Vacant(e) => {
                e.set(Value);
            },
            _ => panic!("Found non-existant entry.")
        }
        assert!(map.contains::<Key, Value>());
    }
}

