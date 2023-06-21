use std::{collections::VecDeque, time::Duration};

mod common;

#[test]
fn test() {
    common::setup_log();
    let pool = executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(3)
        .keep_alive_time(Duration::from_secs(300))
        .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::CallerRuns)
        .build();
    let mut expectations = VecDeque::new();
    let e = 10;
    for i in 0..e {
        let j = i.clone();
        let exp = pool
            .execute(move || {
                log::info!("Run in a thread pool!");
                std::thread::sleep(Duration::from_secs(3));
                j
            })
            .unwrap();

        expectations.push_back(exp);
    }
    for i in 0..e {
        let mut exp = expectations.pop_front().unwrap();
        assert_eq!(exp.get_result().unwrap(), i);
    }
    let f = pool.execute(|| "abc");
    assert_eq!(f.unwrap().get_result().unwrap(), "abc");
}

#[test]
fn test_panic() {
    common::setup_log();
    let pool = executor::ThreadPool::new(1);
    let r = pool.execute(|| {
        panic!("panic!!!");
    });
    let res = r.unwrap().get_result();
    assert!(res.is_err());
    if let Err(err) = res {
        matches!(err.kind(), executor::error::ErrorKind::Panic);
    }

    let r = pool.execute(|| "abc");
    assert_eq!(r.unwrap().get_result().unwrap(), "abc");
}

#[test]
fn test_timeout() {
    common::setup_log();
    let pool = executor::ThreadPool::new(1);
    let r = pool.execute(|| {
        std::thread::sleep(Duration::from_secs(10));
    });
    let res = r.unwrap().get_result_timeout(Duration::from_secs(3));
    assert!(res.is_err());
    if let Err(err) = res {
        matches!(err.kind(), executor::error::ErrorKind::TimeOut);
    }
}
