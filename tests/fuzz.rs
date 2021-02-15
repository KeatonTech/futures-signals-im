use im::Vector;
use rand::random;
use signals_im::hash_map::{MutableHashMap, SignalHashMapExt};
use signals_im::vector::{MutableVector, SignalVectorExt};

mod util;

#[test]
fn fuzz_test_map_signal() {
    let input_map = MutableHashMap::<u8, u8>::new();
    let mut signal = input_map.as_signal().map_values(|v| *v);
    for i in 0..1000 {
        for _ in 0..i {
            let opt = random::<f32>();
            if opt < 0.5 {
                input_map.write().insert(random(), random());
            } else if opt < 0.9 {
                input_map.write().remove(&random());
            } else if opt < 0.95 {
                input_map.write().replace(
                    vec![
                        (random(), random()),
                        (random(), random()),
                        (random(), random()),
                    ]
                    .into_iter(),
                );
            } else {
                input_map.write().clear();
            }
        }

        let poll = util::poll_all(&mut signal);
        let snapshots = util::get_snapshots(&poll.items);
        if snapshots.len() == 0 {
            continue;
        }
        let last_snapshot = snapshots.last().unwrap();
        assert_eq!(*last_snapshot, input_map.read().snapshot());
    }
}

#[test]
fn fuzz_test_vector_signal() {
    let input_vec = MutableVector::<u8>::new();
    let mut signal = input_vec.as_signal().map(|v| *v);
    let mut changes: Vec<String> = vec![];
    let mut last_state: Option<Vector<u8>> = None;

    for i in 0..1000 {
        changes = vec![];
        for _ in 0..i {
            let opt = random::<f32>();
            if opt < 0.2 {
                changes.push("push_back".to_string());
                input_vec.write().push_back(random());
            }
            if opt < 0.3 {
                changes.push("push_front".to_string());
                input_vec.write().push_front(random());
            }
            if opt < 0.5 {
                let len = input_vec.read().len() as f32;
                let idx = (random::<f32>() * len) as usize;
                changes.push(format!("insert at {}", idx));
                input_vec.write().insert(idx, random());
            } else if opt < 0.6 {
                let len = input_vec.read().len() as f32;
                if len == 0f32 {
                    continue;
                }
                let idx = (random::<f32>() * (len - 1f32)) as usize;
                changes.push(format!("remove at {}", idx));
                input_vec.write().remove(idx);
            } else if opt < 0.7 {
                changes.push("pop_back".to_string());
                input_vec.write().pop_back();
            } else if opt < 0.8 {
                changes.push("pop_front".to_string());
                input_vec.write().pop_front();
            } else if opt < 0.9 {
                let len = input_vec.read().len() as f32;
                if len == 0f32 {
                    continue;
                }
                let idx = (random::<f32>() * (len - 1f32)) as usize;
                changes.push(format!("set at {}", idx));
                input_vec.write().set(idx, random());
            } else if opt < 0.95 {
                changes.push("replace".to_string());
                input_vec
                    .write()
                    .replace(vec![random(), random(), random()].into_iter());
            } else {
                changes.push("clear".to_string());
                input_vec.write().clear();
            }
        }

        let poll = util::poll_all(&mut signal);
        let snapshots = util::get_snapshots(&poll.items);
        if snapshots.len() == 0 {
            continue;
        }
        let last_snapshot = snapshots.last().unwrap();
        let change_stack = changes.join("\n * ");
        assert_eq!(
            *last_snapshot,
            input_vec.read().snapshot(),
            "Last State: {:?}\n\nChanges: \n * {}\n\n",
            last_state,
            change_stack
        );
        last_state = Some(last_snapshot.clone());
    }
}
