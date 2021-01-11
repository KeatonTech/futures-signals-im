use im::hashmap;
use signals_im::hash_map::MapDiff;
use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};

mod util;

#[test]
fn map_values() {
    let input_map = MutableHashMap::<u8, u8>::new();
    input_map.write().insert(1, 1);

    let mut multiplied = input_map.as_signal().map_values(|v| v * 2);
    input_map.write().insert(2, 2);

    let poll_1 = util::poll_all(&mut multiplied);
    assert_eq!(poll_1.is_done, false);
    assert_eq!(
        *util::get_snapshots(&poll_1.items).last().unwrap(),
        hashmap! {
            1 => 2,
            2 => 4,
        }
    );
    assert_eq!(
        util::get_hash_map_diffs(&poll_1.items),
        vec![MapDiff::Replace {},]
    );
    input_map.write().insert(2, 2);
    input_map.write().insert(3, 2);
    input_map.write().insert(3, 3);
    input_map.write().insert(4, 4);
    input_map.write().remove(&3);

    let poll_2 = util::poll_all(&mut multiplied);
    assert_eq!(poll_2.is_done, false);
    assert_eq!(
        *util::get_snapshots(&poll_2.items).last().unwrap(),
        hashmap! {
            1 => 2,
            2 => 4,
            4 => 8,
        }
    );
    assert_eq!(
        util::get_hash_map_diffs(&poll_2.items),
        vec![
            MapDiff::Update { key: 2 },
            MapDiff::Insert { key: 4 },
        ]
    );
}
