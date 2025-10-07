use app_config::APP_CONFIG;
use std::env;
use std::path::Path;
use time::macros::format_description as time_format;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::time::LocalTime;

pub fn init_main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::builder().parse_lossy(&APP_CONFIG.rust_log))
        .with_timer(LocalTime::new(time_format!(
            "[year]-[month]-[day] [hour]:[minute]:[second]"
        )))
        .with_level(true)
        .init();
}

pub fn init_test() {
    let workspace_root = env::var("CARGO_MANIFEST_DIR")
        .map(|dir| Path::new(&dir).parent().unwrap().to_path_buf())
        .unwrap();

    env::set_current_dir(&workspace_root).unwrap();

    tracing_subscriber::fmt().with_level(true).init();
}
