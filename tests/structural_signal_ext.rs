use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
use signals_im::vector::MutableVector;
use signals_im::StructuralSignalExt;
use im::{hashmap, vector};

#[test]
fn broadcast_hash_map() {
    let input_map = MutableHashMap::<u8, u8>::new();
    input_map.write().insert(1, 1);

    // This transform will only occur once for each map update, regardless of
    // how many signals the broadcaster creates.
    let broadcaster = input_map.as_signal().map_values(|v| v * 2).broadcast();
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), hashmap!{1 => 2});

    input_map.write().insert(2, 2);
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), hashmap!{1 => 2, 2 => 4});
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), hashmap!{1 => 2, 2 => 4});
}

#[test]
fn broadcast_vector() {
    let input_map = MutableVector::<u8>::new();
    input_map.write().push_back(1);
    input_map.write().push_back(1);
    input_map.write().push_back(2);

    // This transform will only occur once for each map update, regardless of
    // how many signals the broadcaster creates.
    let broadcaster = input_map.as_signal().broadcast();
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), vector![1, 1, 2]);

    input_map.write().insert(0, 0);
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), vector![0, 1, 1, 2]);
    assert_eq!(broadcaster.get_signal().snapshot().unwrap(), vector![0, 1, 1, 2]);
}