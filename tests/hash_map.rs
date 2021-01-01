use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};

#[test]
fn map_values() {
    let input_map = MutableHashMap::<u8, u8>::new();
    input_map.write().insert(1, 1);

    let multiplied = input_map.as_signal().map_values(|v| v * 2);
    input_map.write().insert(2, 2);

    let multiplied_map = multiplied.into_map_sync().unwrap();
    assert_eq!(multiplied_map.get(&1), Some(&2));
    assert_eq!(multiplied_map.get(&2), Some(&4));
    assert_eq!(multiplied_map.get(&3), None);
}