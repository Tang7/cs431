use std::cmp::Ordering::{self, *};
use std::ptr::{null, null_mut};
use std::sync::{Mutex, MutexGuard};
use std::{mem, ptr};

use crate::ConcurrentSet;

#[derive(Debug)]
struct Node<T> {
    data: T,
    next: Mutex<*mut Node<T>>,
}

/// Concurrent sorted singly linked list using fine-grained lock-coupling.
#[derive(Debug)]
pub struct FineGrainedListSet<T> {
    head: Mutex<*mut Node<T>>,
}

unsafe impl<T: Send> Send for FineGrainedListSet<T> {}
unsafe impl<T: Send> Sync for FineGrainedListSet<T> {}

/// Reference to the `next` field of previous node which points to the current node.
///
/// For example, given the following linked list:
///
/// ```text
/// head -> 1 -> 2 -> 3 -> null
/// ```
///
/// If `cursor` is currently at node 2, then `cursor.0` should be the `MutexGuard` obtained from the
/// `next` of node 1. In particular, `cursor.0.as_ref().unwrap()` creates a shared reference to node
/// 2.
struct Cursor<'l, T>(MutexGuard<'l, *mut Node<T>>);

impl<T> Node<T> {
    fn new(data: T, next: *mut Self) -> *mut Self {
        Box::into_raw(Box::new(Self {
            data,
            next: Mutex::new(next),
        }))
    }
}

impl<T: Ord> Cursor<'_, T> {
    /// Moves the cursor to the position of key in the sorted list.
    /// Returns whether the value was found.
    fn find(&mut self, key: &T) -> bool {
        loop {
            let node = *self.0;

            if node.is_null() {
                return false;
            }

            let node = unsafe { &*node };

            match key.cmp(&node.data) {
                Ordering::Less => return false,
                Ordering::Equal => return true,
                Ordering::Greater => self.0 = node.next.lock().unwrap(),
            }
        }
    }
}

impl<T> FineGrainedListSet<T> {
    /// Creates a new list.
    pub fn new() -> Self {
        Self {
            head: Mutex::new(ptr::null_mut()),
        }
    }
}

impl<T: Ord> FineGrainedListSet<T> {
    fn find(&self, key: &T) -> (bool, Cursor<'_, T>) {
        let mut cursor = Cursor(self.head.lock().unwrap());
        let found = cursor.find(key);
        (found, cursor)
    }
}

impl<T: Ord> ConcurrentSet<T> for FineGrainedListSet<T> {
    fn contains(&self, key: &T) -> bool {
        self.find(key).0
    }

    fn insert(&self, key: T) -> bool {
        let (found, mut cursor) = self.find(&key);
        if found {
            return false;
        }

        let node = *cursor.0;
        *cursor.0 = Node::new(key, node);

        true
    }

    fn remove(&self, key: &T) -> bool {
        let (found, mut cursor) = self.find(key);
        if !found {
            return false;
        }

        let node_ptr = *cursor.0;
        if node_ptr.is_null() {
            return false;
        }

        let node = unsafe { &*node_ptr };
        let mut next_guard = node.next.lock().unwrap();
        let next_node_ptr = *next_guard;
        *cursor.0 = next_node_ptr;

        drop(next_guard);
        unsafe {
            let _ = Box::from_raw(node_ptr);
        }

        true
    }
}

#[derive(Debug)]
pub struct Iter<'l, T> {
    cursor: MutexGuard<'l, *mut Node<T>>,
}

impl<T> FineGrainedListSet<T> {
    /// An iterator visiting all elements.
    pub fn iter(&self) -> Iter<'_, T> {
        Iter {
            cursor: self.head.lock().unwrap(),
        }
    }
}

impl<'l, T> Iterator for Iter<'l, T> {
    type Item = &'l T;

    fn next(&mut self) -> Option<Self::Item> {
        let node = *(self.cursor);
        if node.is_null() {
            return None;
        }

        let data = Some(unsafe { &(*node).data });
        self.cursor = unsafe { (*node).next.lock().unwrap() };

        data
    }
}

impl<T> Drop for FineGrainedListSet<T> {
    fn drop(&mut self) {
        let mut current = *self.head.lock().unwrap();

        while !current.is_null() {
            let node = unsafe { Box::from_raw(current) };
            current = *node.next.lock().unwrap();
        }
    }
}

impl<T> Default for FineGrainedListSet<T> {
    fn default() -> Self {
        Self::new()
    }
}
