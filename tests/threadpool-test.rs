mod common;

#[test]
fn test() {
    common::setup_log();
    let pool = executor::ThreadPool::new(1, 10);
    let mut res = pool
        .execute(|| {
            log::info!("Run in a thread pool!");
            4
        })
        .unwrap();
    assert_eq!(res.get_result().unwrap(), 4);
}
