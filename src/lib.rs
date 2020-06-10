use std::rc::Rc;
use std::sync::Mutex;
use std::collections::HashMap;
use std::hash::Hash;

/// A structure for managing a tree of `HashMap`s
///
/// General layout inspired by [A Persistent Singly-Linked Stack](https://rust-unofficial.github.io/too-many-lists/third.html), adapted and extended with `Mutex`es and `HashMap`s
pub struct ChainMap<K, V>
where
  K: Eq + Hash,
  V: Clone {
    head: Link<K, V>,
}

type Link<K, V> = Option<Rc<Node<K, V>>>;

struct Node<K, V>
where
  K: Eq + Hash,
  V: Clone {
    elem: Mutex<HashMap<K, V>>,
    next: Link<K, V>,
}

impl<K, V> ChainMap<K, V>
where
  K: Eq + Hash,
  V: Clone {
    /// Create a new empty root
    pub fn new() -> Self {
        Self { head: Some(Rc::new(Node {
            elem: Mutex::new(HashMap::new()),
            next: None,
        }))}
    }

    /// Create a new branch an put in an empty level
    pub fn extend(&self) -> Self {
        Self { head: Some(Rc::new(Node {
            elem: Mutex::new(HashMap::new()),
            next: self.head.clone(),
        }))}
    }

    /// Create a new branch and initialize it with given map
    pub fn append(&self, elem: HashMap<K, V>) -> Self {
        Self { head: Some(Rc::new(Node {
            elem: Mutex::new(elem),
            next: self.head.clone(),
        }))}
    }

    /// Util only
    fn tail(&self) -> Self {
        Self { head: self.head.as_ref().and_then(|node| node.next.clone()) }
    }

    /// Util only
    fn head(&self) -> Option<&Mutex<HashMap<K, V>>> {
        self.head.as_ref().map(|node| &node.elem)
    }

    /// Create a new binding in the toplevel
    pub fn insert(&self, key: K, val: V) {
        self.head().unwrap().lock().unwrap().insert(key, val);
    }

    ///
    pub fn get(&self, key: &K) -> Option<V> {
        let mut r = &self.head;
        loop {
            if let Some(m) = r {
                match m.elem.lock().unwrap().get(&key) {
                    None => r = &m.next,
                    Some(val) => return Some(val.clone()),
                }
            } else {
                return None;
            }
        }
    }

    pub fn local_get(&self, key: &K) -> Option<V> {
        self.head().unwrap().lock().unwrap().get(&key).map(|v| v.clone())
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
        let ch0 = ChainMap::new();
        let ch1 = ch0.append(h1);
        let ch2 = ch1.append(h2);
        ch0.insert(0, "z0");
        // Note: although this is very ugly, it is only temporary
        assert_eq!(ch1.head().unwrap().lock().unwrap().get(&0), Some(&"a1"));
        assert_eq!(ch2.head().unwrap().lock().unwrap().get(&0), Some(&"a2"));
        let ch3a = ch2.extend();
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
        let ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        ch0.insert(2, "b0");
        let ch1a = ch0.extend();
        ch1a.insert(3, "c1");
        let ch1b = ch0.extend();
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
        let ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        ch0.insert(2, "b0");
        let ch1 = ch0.extend();
        let ch2 = ch1.extend();
        let ch3 = ch2.extend();
        let ch4 = ch3.extend();
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
        let ch0 = ChainMap::new();
        ch0.insert(1, "a0");
        let ch1 = ch0.extend();
        let ch2 = ch1.extend();
        let ch3 = ch2.extend();
        let ch4 = ch3.extend();
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
        let ch0 = ChainMap::new();
        let ch1 = ch0.extend();
        let ch2 = ch1.extend();
        let ch3 = ch2.extend();
        let ch4 = ch3.extend();
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
}
