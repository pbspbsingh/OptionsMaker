use std::env;
use std::path::Path;

pub fn init_test() {
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(|dir| Path::new(&dir).parent().unwrap().to_path_buf())
        .unwrap();

    env::set_current_dir(&workspace_root).unwrap();

    tracing_subscriber::fmt().with_level(true).init();
}
