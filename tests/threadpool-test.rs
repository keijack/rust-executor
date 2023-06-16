use std::{collections::VecDeque, time::Duration};

mod common;

#[test]
fn test() {
    common::setup_log();
    let pool = executor::ThreadPool::new(1, 3, executor::Policy::CallerRuns);
    let mut futures = VecDeque::new();
    let e = 10;
    for i in 0..e {
        let j = i.clone();
        let res = pool
            .execute(move || {
                log::info!("Run in a thread pool!");
                std::thread::sleep(Duration::from_secs(3));
                j
            })
            .unwrap();

        futures.push_back(res);
        std::thread::sleep(Duration::from_secs(i/2));
    }
    for i in 0..e {
        let mut f = futures.pop_front().unwrap();
        assert_eq!(f.get_result().unwrap(), i);
    }
}
