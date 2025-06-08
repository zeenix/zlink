//! List type for holding either borrowed or owned collections.

use serde::{Deserialize, Serialize};

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// A list that can be either borrowed or owned.
///
/// This type is useful for const contexts where we need borrowed data,
/// as well as for deserialization where we need owned data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum List<'a, T> {
    /// Borrowed slice of references, useful for const contexts.
    Borrowed(&'a [&'a T]),
    /// Owned vector, used for deserialization.
    Owned(Vec<T>),
}

impl<'a, T> List<'a, T> {
    /// Returns an iterator over references to the items.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        match self {
            List::Borrowed(slice) => ListIter::Borrowed(slice.iter()),
            List::Owned(vec) => ListIter::Owned(vec.iter()),
        }
    }

    /// Returns the number of items in the list.
    pub fn len(&self) -> usize {
        match self {
            List::Borrowed(slice) => slice.len(),
            List::Owned(vec) => vec.len(),
        }
    }

    /// Returns true if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Iterator over list items.
enum ListIter<'a, T> {
    Borrowed(core::slice::Iter<'a, &'a T>),
    Owned(core::slice::Iter<'a, T>),
}

impl<'a, T> Iterator for ListIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ListIter::Borrowed(iter) => iter.next().copied(),
            ListIter::Owned(iter) => iter.next(),
        }
    }
}

impl<'a, T> Serialize for List<'a, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeSeq;

        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for item in self.iter() {
            seq.serialize_element(item)?;
        }
        seq.end()
    }
}

impl<'de, 'a, T> Deserialize<'de> for List<'a, T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let vec = Vec::<T>::deserialize(deserializer)?;
        Ok(List::Owned(vec))
    }
}

impl<'a, T> Default for List<'a, T> {
    fn default() -> Self {
        List::Borrowed(&[])
    }
}

impl<'a, T> From<Vec<T>> for List<'a, T> {
    fn from(vec: Vec<T>) -> Self {
        List::Owned(vec)
    }
}

impl<'a, T> From<&'a [&'a T]> for List<'a, T> {
    fn from(slice: &'a [&'a T]) -> Self {
        List::Borrowed(slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_borrowed() {
        static ITEM_ONE: &str = "one";
        static ITEM_TWO: &str = "two";
        static ITEM_THREE: &str = "three";
        static ITEMS: [&'static &str; 3] = [&ITEM_ONE, &ITEM_TWO, &ITEM_THREE];
        let list: List<'_, &str> = List::Borrowed(&ITEMS);

        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());

        let collected: Vec<_> = list.iter().collect();
        assert_eq!(collected, vec![&"one", &"two", &"three"]);
    }

    #[test]
    fn test_list_owned() {
        let vec = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let list: List<'_, String> = List::from(vec);

        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());

        let collected: Vec<_> = list.iter().map(|s| s.as_str()).collect();
        assert_eq!(collected, vec!["one", "two", "three"]);
    }

    #[test]
    fn test_serialization() {
        let vec = vec![1, 2, 3];
        let list: List<'_, i32> = List::from(vec);

        let json = serde_json::to_string(&list).unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[test]
    fn test_deserialization() {
        let json = "[1,2,3]";
        let list: List<'_, i32> = serde_json::from_str(json).unwrap();

        match list {
            List::Owned(vec) => assert_eq!(vec, vec![1, 2, 3]),
            _ => panic!("Expected owned list"),
        }
    }
}
