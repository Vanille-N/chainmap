use std::rc::Rc;
use std::sync::Mutex;
use std::collections::HashMap;
use std::hash::Hash;

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
    pub fn new() -> Self {
        Self { head: Some(Rc::new(Node {
            elem: Mutex::new(HashMap::new()),
            next: None,
        }))}
    }

    pub fn extend(&self) -> Self {
        Self { head: Some(Rc::new(Node {
            elem: Mutex::new(HashMap::new()),
            next: self.head.clone(),
        }))}
    }

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
    pub fn insert(&self, key: K, val: V) {
        self.head().unwrap().lock().unwrap().insert(key, val);
    }
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
        // Note: although this is very ugly, it is only temporary
        assert_eq!(ch1.head().unwrap().lock().unwrap().get(&0), Some(&"a1"));
        assert_eq!(ch2.head().unwrap().lock().unwrap().get(&0), Some(&"a2"));
        let ch3a = ch2.extend();
        let ch3b = ch2.extend();
        ch3a.head().unwrap().lock().unwrap().insert(4, "e3a");
        ch2.head().unwrap().lock().unwrap().insert(4, "e2");
        assert_eq!(ch2.head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
        assert_eq!(ch3a.head().unwrap().lock().unwrap().get(&4), Some(&"e3a"));
        assert_eq!(ch3a.tail().head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
        assert_eq!(ch3b.tail().head().unwrap().lock().unwrap().get(&4), Some(&"e2"));
    }
}
