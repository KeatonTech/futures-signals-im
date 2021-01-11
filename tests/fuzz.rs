use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
use rand::random;

mod util;

#[test]
fn fuzz_test_map_signal() {
    let input_map = MutableHashMap::<u8, u8>::new();
    let mut signal = input_map.as_signal().map_values(|v| *v);
    
    for _ in 0..10 {
        for _ in 0..1000 {
            let opt = random::<f32>();
            if opt < 0.5 {
                input_map.write().insert(random(), random());
            } else if opt < 0.9 {
                input_map.write().remove(&random());
            } else if opt < 0.95 {
                input_map.write().replace(vec![
                    (random(), random()),
                    (random(), random()),
                    (random(), random()),
                ].into_iter());
            } else {
                input_map.write().clear();
            }
        }

        let poll = util::poll_all(&mut signal);
        let snapshots = util::get_snapshots(&poll.items);
        let last_snapshot = snapshots.last().unwrap();
        assert_eq!(*last_snapshot, input_map.read().snapshot());
    }
}

#[test]
fn fuzz_test_vector_signal() {
    let input_map = MutableHashMap::<u8, u8>::new();
    let mut signal = input_map.as_signal().map_values(|v| *v);
    
    for _ in 0..10 {
        for _ in 0..1000 {
            let opt = random::<f32>();
            if opt < 0.5 {
                input_map.write().insert(random(), random());
            } else if opt < 0.9 {
                input_map.write().remove(&random());
            } else if opt < 0.95 {
                input_map.write().replace(vec![
                    (random(), random()),
                    (random(), random()),
                    (random(), random()),
                ].into_iter());
            } else {
                input_map.write().clear();
            }
        }

        let poll = util::poll_all(&mut signal);
        let snapshots = util::get_snapshots(&poll.items);
        let last_snapshot = snapshots.last().unwrap();
        assert_eq!(*last_snapshot, input_map.read().snapshot());
    }
}
