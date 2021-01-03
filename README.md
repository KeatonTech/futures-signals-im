# Mutable Data Structures for futures_signals

This library provides an alternate implementation of Mutable data structures from the
`futures_signals` crate, confusingly backed by 'immutable' data structures from the `im`
crate. These backing structures can be easily and efficiently 'snapshotted' without
cloning the whole structure, which allows Signals from data structures to be leaner.

This internal difference is mostly invisible to the user, but does allow for some extra
functionality. In particular, any StructuralSignal (the equivalent of `SignalVec` and
`SignalMap`) can be 'snapshotted', turning it into a normal data structure. This
provides a handy escape hatch for those times when you want to access your data
structures synchronously.

## Example

#### Vectors

```rust
use signals_im::vector::MutableVector;
use signals_im::StructuralSignalExt;
use im::vector;

let input_map = MutableVector::<u8>::new();
input_map.write().push_back(1);
input_map.write().push_back(1);
input_map.write().push_back(2);

input_map.write().insert(0, 0);
assert_eq!(input_map.get_signal().snapshot().unwrap(), vector![0, 1, 1, 2]);
```

#### Hash Maps

```rust
use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
use signals_im::StructuralSignalExt;
use im::hashmap;

let input_map = MutableHashMap::<u8, u8>::new();
input_map.write().insert(1, 1);

// This this map will follow any changes to the input map, but with
// all the values multiplied by 2 automatically.
let multiplied_map = input_map.as_signal().map_values(|v| v * 2);

// The multiplied_map var could be plugged directly into anything that
// takes a StructuralSignal (such as a UI framework). But only once.
// If multiple consumers want access to this map, it can be broadcasted.
let broadcaster = multiplied_map.broadcast();

// Using the `.snapshot()` function to turn the signal into a normal HashMap
assert_eq!(broadcaster.get_signal().snapshot().unwrap(), hashmap!{1 => 2});

// See that the broadcaster updates as the input map changes.
input_map.write().insert(2, 2);
assert_eq!(broadcaster.get_signal().snapshot().unwrap(), hashmap!{1 => 2, 2 => 4});
```

## Status

HashMaps are the most fully realized right now. It's harder to compete with `MutableVec`
in future_signals as it's much more fleshed out and has a lot of cool functionality on
it. This started mostly as a toy concept project but I'm anecdotally seeing some
significant performance benefits over futures_signals maps so I might continue on it.

A good next step would be to figure out an actual benchmark to validate the performance
improvements, and maintain it. Also building out MutableVec more.

There is currently a compatibility layer to allow this library's `MutableVector` to
interoperate with futures_signals' `SignalVec` enough to be used in Dominator. Some
more performance benefits could be obtained by writing actual bindings for Dominator.

Also there needs to be way more tests.