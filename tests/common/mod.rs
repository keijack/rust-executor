pub fn setup_log() {
    if let Err(_) = simple_logger::init_with_level(log::Level::Debug) {
        // ignore error.
    };
}
