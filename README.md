# ChainMap

[![](https://img.shields.io/badge/github-Vanille--N/chainmap-8da0cb?logo=github)](https://github.com/Vanille-N/chainmap)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Codecov](https://img.shields.io/codecov/c/github/Vanille-N/chainmap?logo=codecov)](https://codecov.io/github/Vanille-N/chainmap)

`chainmap` [![](http://meritbadge.herokuapp.com/chainmap)](https://crates.io/crates/chainmap)
[![API](https://docs.rs/chainmap/badge.svg)](https://docs.rs/chainmap)

This library provides a chain of `HashMap`s with interior mutability of each intermediate layer. The `HashMap`s are reference-counted, thus it is possible to create a tree of layers of `HashMap`s and not just a single chain.

The higher maps in the tree (close to the leaves) have higher priority.

## What is this crate good for ?

The implementation allows several maps to read and write to a common root. Not as many items are cloned as when using a `HashMap`. Calls to `insert` have been measured to be about twice as slow as when using a plain `HashMap`, and both `update` and `get` are linear relative to the depth of the `ChainMap`: do not use this crate if what you want to do can be solved with a plain `HashMap`.

On the other hand, you might want to use this if:
- you have many calls to `clone` on a `HashMap` with constant values, in which case you can pass around `ChainMap`s with `map.extend().readonly()`
- you want a `HashMap` shared by multiple objects/threads, then you can use `map.extend()` and make calls to `update` on it
- you have a collection of mappings that you want to override locally without affecting the default values, this can be done with `map.extend()` then calls to `insert`

Possible use cases that group all of the above include:

#### 1. Management of nested scopes for implementing an interpreter

An example from the appropriate section of the Book: 15. Scoping rules - RAII

```rust
fn create_box() {
    // CreateBoxScope
    let _box1 = Box::new(3i32);
}

fn main() {
    // MainScope
    let _box2 = Box::new(5i32);
    {
        // NestedScope
        let box3 = Box::new(4i32);
    }

    for i in 0u32..1_000 {
        // LoopScope<i>
        create_box();
    }
}
```

Could be represented as
```text
MainScope["_box2" => 5i32]
    ├── NestedScope["_box3" => 4i32]
    ├── LoopScope0[]
    │       └── CreateBoxScope["_box1" => 3i32]
    ├── LoopScope1[]
    │       └── CreateBoxScope["_box1" => 3i32]
    │   ...
    └── LoopScope999[]
            └── CreateBoxScope["_box1" => 3i32]
```
Where each `[ $( $key => $value ),* ]` is a level of a tree of `ChainMap`s built on the previous one.

This it turn could be declared as
```rust
let mut main_scope = ChainMap::new();
main_scope.insert("_box2", 5i32);

let mut nested_scope = main_scope.extend();
nested_scope.insert("_box1", 5i32);

let mut loop_scope = Vec::new();
for _ in 0..1000 {
    let mut h = HashMap::new();
    h.insert("_box1", 3i32);
    loop_scope.push(main_scope.extend().extend_with(h));
}
```

The rules for which map entries are accessible from a certain level of the `ChainMap` tree are exactly the same as how they would be for the corresponding scopes.

#### 2. Configuration files at different levels

For example with `cargo`:
```text
~
├── proj/
│    ├── foo/
│    │    ├── bar/
│    │    │    └── .cargo/config
│    │    ├── baz/
│    │    │    └── .cargo/config
│    │    └── .cargo/config
│    ├── quux/
│    │    └── .cargo/config
│    └── .cargo/config
└── .cargo/config
```

The `config` files closer to the leaves have higher priority and can override the configuration settings defined closer to the root in the directory tree.

This situation can be trivially abstracted with a `ChainMap`:
```rust
let home = ChainMap::new();
let proj = home.extend();
let foo = proj.extend();
let quux = proj.extend();
let bar = foo.extend();
let baz = foo.extend();
```

## Why another chain map ?

There are already chain maps out there:

`chain-map` [![](http://meritbadge.herokuapp.com/chain-map)](https://crates.io/crates/chain-map)

`hash-chain` [![](http://meritbadge.herokuapp.com/hash-chain)](https://crates.io/crates/hash-chain)

However, both of these implementations of a chain map do not allow multiple branches from a single root, as they are wrappers around a `Vec<HashMap<K, V>>`.

On the other hand, this crate allows one to fork several maps out of a common root, saving memory usage at the cost of a less friendly internal representation: A `Vec<HashMap<K, V>>` is certainly better to work with than a tree of `Option<Rc<Mutex<HashMap<K, V>>>>`.

## Why require `mut` everywhere if there is interior mutability ?

The `ChainMap` could just as well take `&self` everywhere instead of requiring `&mut self`, and it would still work. After all, a `Mutex` can have its contents changed even if its container is immutable.

There are two reasons for not making all methods take `&self`:

1. Despite interior mutability, it would feel weird to `insert` into a non-`mut` structure.

    A `HashMap` requires `mut` to `insert`, and I wanted the `ChainMap` to feel like a `HashMap` as much as possible, hence the choice of the same method names `insert` and `get`.

2. The `fork` and `fork_with` methods do require `&mut self` and there is no (safe) way to bypass that.

    `fork` is declared as:
    ```rust
    pub fn fork(&mut self) -> Self {
        let newlevel = self.extend();
        let oldlevel = self.extend_fallthrough();
        // This line requires &mut self
        std::mem::replace(&mut *self, oldlevel);
        newlevel
    }
    ```
    When used:
    ```rust
    let ch = ChainMap::new();
    let _ = ch.fork();
    ```
    `ch0` is not the same object before and after the call to `fork` !

    The object that used to be contained in `ch` has been moved out and there is now no way to access the former `ch` other than implicitly by reading it from one of its children.

    It is also impossible to insert a new key into it.
