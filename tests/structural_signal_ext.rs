use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
use signals_im::StructuralSignalExt;
use im::hashmap;

#[test]
fn broadcast() {
    let input_map = MutableHashMap::<u8, u8>::new();
    input_map.write().insert(1, 1);

    // This transform will only occur once for each map update, regardless of
    // how many signals the broadcaster creates.
    let broadcaster = input_map.as_signal().map_values(|v| v * 2).broadcast();
    assert_eq!(broadcaster.get_signal().into_map_sync().unwrap(), hashmap!{1 => 2});

    input_map.write().insert(2, 2);
    assert_eq!(broadcaster.get_signal().into_map_sync().unwrap(), hashmap!{1 => 2, 2 => 4});
    assert_eq!(broadcaster.get_signal().into_map_sync().unwrap(), hashmap!{1 => 2, 2 => 4});
}