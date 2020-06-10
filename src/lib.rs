use std::rc::Rc;
use std::sync::Mutex;
use std::collections::HashMap;
use std::hash::Hash;

/// A structure for managing a tree of `HashMap`s
///
/// General layout inspired by [A Persistent Singly-Linked Stack](https://rust-unofficial.github.io/too-many-lists/third.html), adapted and extended with `Mutex`es and `HashMap`s
pub struct ChainMap<K, V>
where
  K: Eq + Hash + Clone,
  V: Clone {
    head: Link<K, V>,
}

type Link<K, V> = Option<Rc<Node<K, V>>>;

struct Node<K, V>
where
  K: Eq + Hash + Clone,
  V: Clone {
    elem: Mutex<HashMap<K, V>>,
    next: Link<K, V>,
    fallthrough: bool,
}

impl<K, V> ChainMap<K, V>
where
  K: Eq + Hash + Clone,
  V: Clone {
    /// Util only
    #[allow(dead_code)]
    fn tail(&self) -> Self {
        Self {
            head: self.head.as_ref().and_then(|node| node.next.clone()),
        }
    }

    /// Util only
    #[allow(dead_code)]
    fn head(&self) -> Option<&Mutex<HashMap<K, V>>> {
        self.head.as_ref().map(|node| &node.elem)
    }

    /// Create a new empty root
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            head: Some(Rc::new(Node {
                elem: Mutex::new(HashMap::new()),
                next: None,
                fallthrough: false,
            })),
        }
    }

    /// Create a new root and initialize with given map
    pub fn new_with(h: HashMap<K, V>) -> Self {
        Self {
            head: Some(Rc::new(Node {
                elem: Mutex::new(h),
                next: None,
                fallthrough: false,
            })),
        }
    }

    /// Create a new binding in the toplevel
    pub fn insert(&mut self, key: K, val: V) {
        self.head().unwrap().lock().unwrap().insert(key, val);
    }

    /// Retrieve value associated with the first appearance of `key` in the chain
    pub fn get(&self, key: &K) -> Option<V> {
        let mut r = &self.head;
        while let Some(m) = r {
            match m.elem.lock().unwrap().get(&key) {
                None => r = &m.next,
                Some(val) => return Some(val.clone()),
            }
        }
        None
    }

    /// Check associated value only in topmost maps: stops at the first non-fallthrough level
    pub fn local_get(&self, key: &K) -> Option<V> {
        let mut r = &self.head;
        while let Some(m) = r {
            match m.elem.lock().unwrap().get(&key) {
                None => {
                    if m.fallthrough {
                        r = &m.next;
                    } else {
                        return None;
                    }
                }
                Some(val) => return Some(val.clone()),
            }
        }
        None
    }

    /// Replace old value with new, panics if `key` does not exist
    pub fn update(&mut self, key: &K, newval: V) {
        let mut r = &self.head;
        while let Some(m) = r {
            match m.elem.lock().unwrap().get_mut(&key) {
                None => r = &m.next,
                Some(val) => {
                    *val = newval;
                    return;
                }
            }
        }
        panic!("Key does not exist, failed to update");
    }

    /// Replace old value with new, create binding in topmost map if `key` does not exist
    pub fn update_or(&mut self, key: &K, newval: V) {
        let mut r = &self.head;
        while let Some(m) = r {
            match m.elem.lock().unwrap().get_mut(&key) {
                None => r = &m.next,
                Some(val) => {
                    *val = newval;
                    return;
                }
            }
        }
        self.insert(key.clone(), newval);
    }

    pub fn fork(&mut self) -> Self {
        let newlevel = self.extend();
        let oldlevel = self.extend_fallthrough();
        std::mem::replace(&mut *self, oldlevel);
        newlevel
    }

    /// `fork` and `fork_with` are the same as `extend` and `extend_with`,
    /// but later bindings made to `self` are not visible to the other branches (non-fallthrough)
    ///
    /// ```
    /// # use chainmap::*;
    /// # use std::collections::HashMap;
    /// # macro_rules! map {
    /// #     ( $( $key:expr => $val:expr ),* ) => {
    /// #         { let mut h = HashMap::new();
    /// #           $( h.insert($key, $val); )*
    /// #           h
    /// #         }
    /// #     }
    /// # }
    /// let mut root = ChainMap::new_with(map![0 => 'a']);
    /// let layer = root.fork_with(map![1 => 'b']);
    /// root.insert(2, 'c');
    /// ```
    /// ```text
    /// ┌─────────┐    ┌─────────┐
    /// │ ex-root └────┘ layer   │
    /// │           ⇇            │
    /// │ 0 -> a  ┌────┐ 1 -> b  │
    /// └──┐   ┌──┘    └─────────┘
    ///    │ ⇈ │
    /// ┌──┘   └──┐
    /// │  root   │
    /// │         │
    /// │ 2 -> c  │
    /// └─────────┘
    /// ```
    /// ```
    /// # use chainmap::*;
    /// # use std::collections::HashMap;
    /// # macro_rules! map {
    /// #     ( $( $key:expr => $val:expr ),* ) => {
    /// #         { let mut h = HashMap::new();
    /// #           $( h.insert($key, $val); )*
    /// #           h
    /// #         }
    /// #     }
    /// # }
    /// # let mut root = ChainMap::new_with(map![0 => 'a']);
    /// # let layer = root.fork_with(map![1 => 'b']);
    /// # root.insert(2, 'c');
    /// # macro_rules! assert {
    /// #     ( $m:tt[$k:tt] is None ) => { assert_eq!($m.get(&$k), None); };
    /// #     ( $m:tt[$k:tt] is $c:tt ) => { assert_eq!($m.get(&$k), Some($c)); };
    /// # }
    /// assert!(root[0] is 'a');
    /// assert!(layer[0] is 'a');
    ///
    /// assert!(root[1] is None);
    /// assert!(layer[1] is 'b');
    ///
    /// assert!(root[2] is 'c');
    /// assert!(layer[2] is None);
    ///```
    pub fn fork_with(&mut self, h: HashMap<K, V>) -> Self {
        let newlevel = self.extend_with(h);
        let oldlevel = self.extend_fallthrough();
        std::mem::replace(&mut *self, oldlevel);
        newlevel
    }
}

#[cfg(test)]
mod test {
    use super::*;
    macro_rules! map {
        ( $( $key:expr => $val:expr ),* ) => {
            { let mut h = HashMap::new();
              $( h.insert($key, $val); )*
              h
            }
        }
    }

    #[test]
    fn basics() {
        let h1 = map![0 => "a1", 1 => "b1", 2 => "c1"];
        let h2 = map![0 => "a2", 3 => "d2"];
        let mut ch0 = ChainMap::new();
        let ch1 = ch0.extend_with(h1);
        let mut ch2 = ch1.extend_with(h2);
        ch0.insert(0, "z0");
        // Note: although this is very ugly, it is only visible internally
        // The exposed API is a lot more friendly.
        assert_eq!(ch1.head().unwrap().lock().unwrap().get(&0), Some(&"a1"));
        assert_eq!(ch2.head().unwrap().lock().unwrap().get(&0), Some(&"a2"));
        let mut ch3a = ch2.extend();
        let ch3b = ch2.extend();
        ch3a.insert(4, "e3a");
        ch2.insert(4, "e2");
        assert_eq!(ch2.head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
        assert_eq!(ch3a.head().unwrap().lock().unwrap().get(&4), Some(&"e3a"));
        assert_eq!(ch3a.tail().head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
        assert_eq!(ch3b.tail().head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
    }

    #[test]
    fn insert_and_get() {
        let mut ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        ch0.insert(2, "b0");
        let mut ch1a = ch0.extend();
        ch1a.insert(3, "c1");
        let mut ch1b = ch0.extend();
        ch1b.insert(4, "d1");
        assert_eq!(ch0.get(&1), Some("a0"));
        assert_eq!(ch1a.get(&3), Some("c1"));
        assert_eq!(ch1b.get(&4), Some("d1"));
        assert_eq!(ch0.get(&3), None);
        assert_eq!(ch1b.get(&3), None);
        assert_eq!(ch1a.get(&4), None);
    }

    #[test]
    fn deep_get() {
        let mut ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        ch0.insert(2, "b0");
        let ch1 = ch0.extend();
        let ch2 = ch1.extend();
        let ch3 = ch2.extend();
        let mut ch4 = ch3.extend();
        assert_eq!(ch4.get(&1), Some("a0"));
        assert_eq!(ch4.get(&2), Some("b0"));
        assert_eq!(ch4.get(&3), None);
        ch4.insert(3, "c4");
        assert_eq!(ch4.get(&3), Some("c4"));
        assert_eq!(ch3.get(&3), None);
        assert_eq!(ch2.get(&3), None);
        assert_eq!(ch1.get(&3), None);
        assert_eq!(ch0.get(&3), None);
    }

    #[test]
    fn local_get() {
        let mut ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        let ch1 = ch0.extend();
        let ch2 = ch1.extend();
        let ch3 = ch2.extend();
        let mut ch4 = ch3.extend();
        assert_eq!(ch4.local_get(&1), None);
        assert_eq!(ch4.local_get(&3), None);
        ch4.insert(3, "c4");
        assert_eq!(ch4.local_get(&3), Some("c4"));
        assert_eq!(ch3.local_get(&3), None);
        assert_eq!(ch2.local_get(&3), None);
        assert_eq!(ch1.local_get(&3), None);
        assert_eq!(ch0.local_get(&3), None);
        assert_eq!(ch0.local_get(&1), Some("a0"));
    }

    #[test]
    fn override_insert() {
        let mut ch0 = ChainMap::new();
        let mut ch1 = ch0.extend();
        let mut ch2 = ch1.extend();
        let mut ch3 = ch2.extend();
        let mut ch4 = ch3.extend();
        ch0.insert(0, "0");
        ch1.insert(0, "1");
        ch2.insert(0, "2");
        ch3.insert(0, "3");
        ch4.insert(0, "4");
        assert_eq!(ch0.get(&0), Some("0"));
        assert_eq!(ch1.get(&0), Some("1"));
        assert_eq!(ch2.get(&0), Some("2"));
        assert_eq!(ch3.get(&0), Some("3"));
        assert_eq!(ch4.get(&0), Some("4"));
    }

    #[test]
    fn update() {
        let mut ch0 = ChainMap::new_with(map![0 => 'a']);
        let mut ch1a = ch0.extend();
        let mut ch1b = ch0.extend_with(map![0 => 'b']);
        let mut ch2 = ch1a.extend_with(map![0 => 'c']);
        assert_eq!(ch0.get(&0), Some('a'));
        assert_eq!(ch1a.get(&0), Some('a'));
        assert_eq!(ch1b.get(&0), Some('b'));
        assert_eq!(ch2.get(&0), Some('c'));
        ch0.update(&0, 'd');
        assert_eq!(ch0.get(&0), Some('d'));
        assert_eq!(ch1a.get(&0), Some('d'));
        assert_eq!(ch1b.get(&0), Some('b'));
        assert_eq!(ch2.get(&0), Some('c'));
        ch1a.update(&0, 'e');
        assert_eq!(ch0.get(&0), Some('e'));
        assert_eq!(ch1a.get(&0), Some('e'));
        assert_eq!(ch1b.get(&0), Some('b'));
        assert_eq!(ch2.get(&0), Some('c'));
        ch1b.update(&0, 'f');
        assert_eq!(ch0.get(&0), Some('e'));
        assert_eq!(ch1a.get(&0), Some('e'));
        assert_eq!(ch1b.get(&0), Some('f'));
        assert_eq!(ch2.get(&0), Some('c'));
        ch2.update(&0, 'g');
        assert_eq!(ch0.get(&0), Some('e'));
        assert_eq!(ch1a.get(&0), Some('e'));
        assert_eq!(ch1b.get(&0), Some('f'));
        assert_eq!(ch2.get(&0), Some('g'));
    }

    #[test]
    #[should_panic]
    fn update_missing() {
        let mut ch0 = ChainMap::new();
        let _ = ch0.extend_with(map![0 => 'a']);
        ch0.update(&0, 'b');
    }

    #[test]
    fn update_or() {
        let mut ch0 = ChainMap::new();
        let ch1 = ch0.extend_with(map![0 => 'a']);
        ch0.update_or(&0, 'b');
        assert_eq!(ch0.get(&0), Some('b'));
        assert_eq!(ch1.get(&0), Some('a'));
    }

    #[test]
    fn fork() {
        let mut ch0 = ChainMap::new_with(map![0 => 'a']);
        let ch1 = ch0.fork();
        ch0.insert(1, 'b');
        assert_eq!(ch0.get(&1), Some('b'));
        assert_eq!(ch0.local_get(&1), Some('b'));
        assert_eq!(ch1.get(&1), None);
        assert_eq!(ch1.local_get(&1), None);
        assert_eq!(ch0.get(&0), Some('a'));
        assert_eq!(ch0.local_get(&0), Some('a'));
        assert_eq!(ch1.get(&0), Some('a'));
        assert_eq!(ch1.local_get(&0), None);
        ch0.update(&0, 'c');
        assert_eq!(ch0.get(&0), Some('c'));
        assert_eq!(ch0.local_get(&0), Some('c'));
        assert_eq!(ch1.get(&0), Some('c'));
        assert_eq!(ch1.local_get(&0), None);
        ch0.insert(1, 'd');
        assert_eq!(ch0.get(&1), Some('d'));
        assert_eq!(ch0.local_get(&1), Some('d'));
        assert_eq!(ch1.get(&1), None);
        assert_eq!(ch1.local_get(&1), None);
    }

}
