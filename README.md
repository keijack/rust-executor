# Rust Threadpool Executor

[![crate.io-version](https://img.shields.io/crates/v/threadpool-executor)](https://crates.io/crates/threadpool-executor/)
[![crate.io-download](https://img.shields.io/crates/d/threadpool-executor)](https://crates.io/crates/threadpool-executor/)
[![docs.rs-build](https://img.shields.io/docsrs/threadpool-executor)](https://docs.rs/threadpool-executor/latest/threadpool_executor/)

A simple thread pool for running jobs on the worker threads. You can specify the core workers which will live as long as the thread pool , maximum workers which will live with a given keep alive time, and the policy when the jobs submited exceed the maximum size of the workers. 


## Usage

Create a fix size thread pool and when the job submited will wait when all workers are busy:

```rust
let pool = threadpool_executor::ThreadPool::new(1);
let mut expectation = pool.execute(|| {"hello, thread pool!"}).unwrap();
assert_eq!(expectation.get_result().unwrap(), "hello, thread pool!");
```

You can handle wait the result for a specifid time:

```rust
let pool = threadpool_executor::ThreadPool::new(1);
let r = pool.execute(|| {
    std::thread::sleep(std::time::Duration::from_secs(10));
});
let res = r.unwrap().get_result_timeout(std::time::Duration::from_secs(3));
assert!(res.is_err());
if let Err(err) = res {
    matches!(err.kind(), threadpool_executor::error::ErrorKind::TimeOut);
}
```


Use `Builder` to create a thread pool:

```rust
let pool = threadpool_executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(3)
        .keep_alive_time(std::time::Duration::from_secs(300))
        .exeed_limit_policy(threadpool_executor::threadpool::ExceedLimitPolicy::Wait)
        .build();
```

The workers runs in this thread pool will try to catch the `Panic!` using the `std::panic::catch_unwind` in the functions you submit, if catched, the `get_result` method will give you a `Panic` kind ExecutorError.

```rust
let pool = threadpool_executor::ThreadPool::new(1);
let r = pool.execute(|| {
    panic!("panic!!!");
});
let res = r.unwrap().get_result();
assert!(res.is_err());
if let Err(err) = res {
    matches!(err.kind(), threadpool_executor::error::ErrorKind::Panic);
}
```

You can cancel the task when it's waiting in line. The task cannot be cancelled when it's started running. 

```rust
let pool = threadpool_executor::ThreadPool::new(1);
pool.execute(|| {
    std::thread::sleep(std::time::Duration::from_secs(3));
}).unwrap();
let mut exp = pool.execute(|| {}).unwrap();
exp.cancel();
```

