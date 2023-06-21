# Rust Executor

A simple thread pool for running jobs on the worker threads. You can specify the core workers which will live as long as the thread pool , maximum workers which will live with a given keep alive time, and the policy when the jobs submited exceed the maximum size of the workers. 

## Usage

Create a fix size thread pool and when the job submited will wait when all workers are busy:

```rust
let pool = executor::ThreadPool::new(1);
let expectation = pool.execute(|| {"hello, thread pool!"}).unwrap();
assert(expectation.get_result().unwrap(), "hello, thread pool!");
```

You can handle wait the result for a specifid time:

```rust
let pool = executor::ThreadPool::new(1);
let r = pool.execute(|| {
    std::thread::sleep(Duration::from_secs(10));
});
let res = r.unwrap().get_result_timeout(Duration::from_secs(3));
assert!(res.is_err());
if let Err(err) = res {
    matches!(err.kind(), executor::error::ErrorKind::TimeOut);
}
```


Use `Builder` to create a thread pool:

```rust
let pool = executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(3)
        .keep_alive_time(Duration::from_secs(300))
        .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::Wait)
        .build();
```

The workers runs in this thread pool will try to catch the `Panic!` using the `std::panic::catch_unwind` in the functions you submit, if catched, the `get_result` method will give you a `Panic` kind ExecutorError.

```rust
let pool = executor::ThreadPool::new(1);
let r = pool.execute(|| {
    panic!("panic!!!");
});
let res = r.unwrap().get_result();
assert!(res.is_err());
if let Err(err) = res {
    matches!(err.kind(), executor::error::ErrorKind::Panic);
}
```
