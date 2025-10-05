use anyhow::Context;
use app_config::CRAWLER_CONF;
use headless_chrome::Browser;
use std::io::{BufRead, BufReader};
use std::net::{Ipv4Addr, TcpListener};
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::{fs, thread};
use tracing::{info, warn};

const PID_FILE: &str = "chrome.pid";

pub fn init_browser() -> anyhow::Result<Browser> {
    match try_connect_existing_session() {
        Ok(browser) => Ok(browser),
        Err(e) => {
            warn!("Couldn't connect to existing session: {e}");
            start_new_session()
        }
    }
}

fn try_connect_existing_session() -> anyhow::Result<Browser> {
    if !fs::exists(PID_FILE)? {
        anyhow::bail!("'{PID_FILE}' file doesn't exist");
    }

    let pid_file = fs::canonicalize(PID_FILE).context("Failed to canonicalize PID file")?;
    let ws_url = fs::read_to_string(PID_FILE)
        .with_context(|| format!("Failed to read PID file: {}", pid_file.display()))?;

    info!("Connecting to existing session with url: '{ws_url}'");
    match Browser::connect(ws_url.trim().to_owned()) {
        Ok(browser) => {
            info!("Successfully connected to existing browser session");
            Ok(browser)
        }
        Err(e) => {
            warn!("Failed to connect to existing session: {e}");
            fs::remove_file(pid_file)?;
            Err(e)
        }
    }
}

fn start_new_session() -> anyhow::Result<Browser> {
    fn start_chrome_process() -> anyhow::Result<String> {
        let port = quick_port()?;
        info!("Starting new chrome session with remote debugging port at: {port}");
        let mut process = Command::new(&CRAWLER_CONF.chrome_path)
            .arg(format!("--remote-debugging-port={port}"))
            .arg(&CRAWLER_CONF.chrome_extra_args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()?;
        info!("Started a chrome instance with pid: {}", process.id());
        if let Some(output) = process.stderr.take() {
            let mut reader = BufReader::new(output);
            let mut buff = String::new();
            loop {
                reader.read_line(&mut buff)?;
                if buff.starts_with("DevTools listening on") {
                    let ws_url = buff.trim_start_matches("DevTools listening on").trim();
                    fs::write(PID_FILE, ws_url)?;
                    return Ok(ws_url.to_owned());
                }

                buff.clear();
                thread::sleep(Duration::from_millis(200));
            }
        }

        warn!("Couldn't get the stdout of child process");
        process.kill()?;
        anyhow::bail!("Failed to get stdout of child process")
    }

    let ws_url = start_chrome_process()?;
    let browser = Browser::connect(ws_url.clone())
        .with_context(|| format!("Failed to connect to {ws_url}"))?;
    Ok(browser)
}

fn quick_port() -> anyhow::Result<u16> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
        .local_addr()?
        .port())
}
