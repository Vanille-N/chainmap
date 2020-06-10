# ChainMap

[![](https://img.shields.io/badge/github-Vanille--N/chainmap-8da0cb?logo=github)](https://github.com/Vanille-N/chainmap)
[![](http://meritbadge.herokuapp.com/chainmap)](https://crates.io/crates/chainmap)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![API](https://docs.rs/chainmap/badge.svg)](https://docs.rs/chainmap)

This library provides a chain of `HashMap`s with interior mutability of each intermediate `HashMap`. The `HashMap`s are reference-counted, meaning that it is possible to create a tree of layers of `HashMap`s and not just a single chain.

The higher maps in the tree (close to the leaves) have higher priority.

One possible use case is for the management of nested scopes.

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
```
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
let main_scope = ChainMap::new();
main_scope.insert("_box2", 5i32);

let nested_scope = main_scope.extend();
nested_scope.insert("_box1", 5i32);

let mut loop_scope = Vec::new();
for _ in 0..1000 {
    let mut h = HashMap::new();
    h.insert("_box1", 3i32);
    loop_scope.push(main_scope.extend().append(h));
}
```



The rules for which map entries are accessible from a certain level of the `ChainMap` tree are exactly the same as how they would be for the corresponding scopes.
