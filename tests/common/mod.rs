pub fn setup_log() {
    simple_logger::init_with_level(log::Level::Debug).unwrap();
}
