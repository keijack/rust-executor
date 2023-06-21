# Rust Executor

这是一个简单的线程池 rust 实现，支持设定核心线程数（线程会一直运行到线程池被 drop），最大线程数（可以配置非核心线程的存活时间），任务超出最大线程数时的执行策略。同时，你可以等待线程池的执行结果。

## 使用方法

创建一个固定数量的线程池，并且当任务超额后等待旧任务执行完成：

```rust
let pool = executor::ThreadPool::new(1);
let expectation = pool.execute(|| {"hello, thread pool!"}).unwrap();
assert(expectation.get_result().unwrap(), "hello, thread pool!");
```

等待结果时可以选择超时时间。

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


使用 `Builder` 来创建线程池：

```rust
let pool = executor::threadpool::Builder::new()
        .core_pool_size(1)
        .maximum_pool_size(3)
        .keep_alive_time(Duration::from_secs(300))
        .exeed_limit_policy(executor::threadpool::ExceedLimitPolicy::Wait)
        .build();
```

线程池会尝试使用 `std::panic::catch_unwind` 来捕获提交任务的 `Panic`，如果捕获到，则会在 `get_result` 中返回一个 `Panic` 类型的错误。

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

