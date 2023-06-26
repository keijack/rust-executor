use std::{
    collections::VecDeque,
    sync::{atomic::AtomicBool, Arc},
    time::Duration,
};

mod common;

#[test]
fn test() {
    common::setup_log();
    let pool = threadpool_executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(3)
        .keep_alive_time(Duration::from_secs(300))
        .exeed_limit_policy(threadpool_executor::threadpool::ExceedLimitPolicy::CallerRuns)
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
    let pool = threadpool_executor::ThreadPool::new(1);
    let r = pool.execute(|| {
        panic!("panic!!!");
    });
    let res = r.unwrap().get_result();
    assert!(res.is_err());
    if let Err(err) = res {
        matches!(err.kind(), threadpool_executor::error::ErrorKind::Panic);
    }

    let r = pool.execute(|| "abc");
    assert_eq!(r.unwrap().get_result().unwrap(), "abc");
}

#[test]
fn test_timeout() {
    common::setup_log();
    let pool = threadpool_executor::ThreadPool::new(1);
    let r = pool.execute(|| {
        std::thread::sleep(Duration::from_secs(3));
    });
    let res = r.unwrap().get_result_timeout(Duration::from_secs(1));
    assert!(res.is_err());
    if let Err(err) = res {
        matches!(err.kind(), threadpool_executor::error::ErrorKind::TimeOut);
    }
}

#[test]
fn test_worker_exit() {
    common::setup_log();
    let pool = threadpool_executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(2)
        .keep_alive_time(Duration::from_secs(1))
        .build();
    pool.execute(|| std::thread::sleep(Duration::from_secs(1)))
        .unwrap();
    pool.execute(|| std::thread::sleep(Duration::from_secs(1)))
        .unwrap();

    std::thread::sleep(Duration::from_secs(10));
    assert_eq!(1, pool.size());
}

#[test]
fn test_reject() {
    common::setup_log();
    let pool = threadpool_executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(1)
        .exeed_limit_policy(threadpool_executor::threadpool::ExceedLimitPolicy::Reject)
        .build();
    let res = pool.execute(|| {
        std::thread::sleep(std::time::Duration::from_secs(3));
    });
    assert!(res.is_ok());
    let res = pool.execute(|| "a");
    assert!(res.is_err());
    if let Err(err) = res {
        matches!(
            err.kind(),
            threadpool_executor::error::ErrorKind::TaskRejected
        );
    }
}

#[test]
fn test_cancel() {
    common::setup_log();
    let pool = threadpool_executor::ThreadPool::new(1);

    let a = Arc::new(AtomicBool::new(true));
    let a1 = a.clone();
    let mut r1 = pool
        .execute(move || {
            std::thread::sleep(Duration::from_secs(3));
            a1.store(false, std::sync::atomic::Ordering::Relaxed);
        })
        .unwrap();
    std::thread::sleep(Duration::from_secs(1));
    let r1c = r1.cancel();
    assert!(r1c.is_err());
    if let Err(err) = r1c {
        matches!(
            err.kind(),
            threadpool_executor::error::ErrorKind::TaskRunning
        );
    }

    let b = Arc::new(AtomicBool::new(true));
    let b1 = b.clone();
    let mut r2 = pool
        .execute(move || b1.store(false, std::sync::atomic::Ordering::Relaxed))
        .unwrap();
    r2.cancel().unwrap();
    std::thread::sleep(Duration::from_secs(5));
    assert!(r2.is_cancelled());
    assert!(b.load(std::sync::atomic::Ordering::Relaxed));
    assert!(r1.is_done());
    assert!(!a.load(std::sync::atomic::Ordering::Relaxed));
}
