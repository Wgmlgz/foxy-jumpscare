use rand::Rng;
use serde::Serialize;
use std::{
    env,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter, State};
use tiny_http::{Method, Response, Server, StatusCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RunMode {
    Once,
    Random,
    Daemon,
}

#[derive(Clone, Debug)]
struct CliConfig {
    mode: RunMode,
    delay_ms: u64,
    random_min_ms: u64,
    random_max_ms: u64,
    random_count: u64,
    daemon_bind: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            mode: RunMode::Once,
            delay_ms: 0,
            random_min_ms: 5000,
            random_max_ms: 30000,
            random_count: 1,
            daemon_bind: "127.0.0.1:15151".to_string(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct StartPayload {
    close_on_end: bool,
}

#[derive(Default)]
struct RuntimeState {
    frontend_ready: AtomicBool,
    pending: Mutex<Vec<StartPayload>>,
}

fn usage_text() -> &'static str {
    r#"Foxy Jumpscare CLI options:
  --help, -h               Show this help.
  --mode <once|random|daemon>
  --once                   Shortcut for --mode once.
  --random                 Shortcut for --mode random.
  --daemon                 Shortcut for --mode daemon.
  --delay-ms <ms>          Delay before first jumpscare in once mode.
  --random-min-ms <ms>     Minimum random wait between jumpscares.
  --random-max-ms <ms>     Maximum random wait between jumpscares.
  --random-count <n>       Number of random jumpscares (0 = infinite).
  --daemon-bind <addr>     HTTP bind address for daemon mode (default 127.0.0.1:15151).

Examples:
  ./app --delay-ms 8000
  ./app --random --random-min-ms 10000 --random-max-ms 45000 --random-count 5
  ./app --daemon --daemon-bind 0.0.0.0:15151"#
}

fn parse_u64_arg(flag: &str, value: Option<String>) -> Result<u64, String> {
    let raw = value.ok_or_else(|| format!("Missing value for {flag}"))?;
    raw.parse::<u64>()
        .map_err(|_| format!("Invalid value for {flag}: {raw}"))
}

fn parse_mode_arg(value: Option<String>) -> Result<RunMode, String> {
    let raw = value.ok_or_else(|| "Missing value for --mode".to_string())?;
    match raw.as_str() {
        "once" => Ok(RunMode::Once),
        "random" => Ok(RunMode::Random),
        "daemon" => Ok(RunMode::Daemon),
        _ => Err(format!("Invalid value for --mode: {raw}")),
    }
}

fn parse_cli_config() -> Result<CliConfig, String> {
    let mut cfg = CliConfig::default();
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}", usage_text());
                std::process::exit(0);
            }
            "--mode" => cfg.mode = parse_mode_arg(args.next())?,
            "--once" => cfg.mode = RunMode::Once,
            "--random" => cfg.mode = RunMode::Random,
            "--daemon" => cfg.mode = RunMode::Daemon,
            "--delay-ms" => cfg.delay_ms = parse_u64_arg("--delay-ms", args.next())?,
            "--random-min-ms" => cfg.random_min_ms = parse_u64_arg("--random-min-ms", args.next())?,
            "--random-max-ms" => cfg.random_max_ms = parse_u64_arg("--random-max-ms", args.next())?,
            "--random-count" => cfg.random_count = parse_u64_arg("--random-count", args.next())?,
            "--daemon-bind" => {
                cfg.daemon_bind = args
                    .next()
                    .ok_or_else(|| "Missing value for --daemon-bind".to_string())?
            }
            _ => return Err(format!("Unknown argument: {arg}")),
        }
    }

    if cfg.random_min_ms > cfg.random_max_ms {
        return Err("random-min-ms must be <= random-max-ms".to_string());
    }

    Ok(cfg)
}

fn queue_or_emit_start(
    app_handle: &AppHandle,
    runtime_state: &Arc<RuntimeState>,
    payload: StartPayload,
) {
    if runtime_state.frontend_ready.load(Ordering::SeqCst) {
        if let Err(error) = app_handle.emit("foxy:start", payload) {
            eprintln!("Failed to emit start event: {error}");
        }
        return;
    }

    let mut pending = runtime_state
        .pending
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    pending.push(payload);
}

fn spawn_once_mode(app_handle: AppHandle, runtime_state: Arc<RuntimeState>, delay_ms: u64) {
    thread::spawn(move || {
        if delay_ms > 0 {
            thread::sleep(Duration::from_millis(delay_ms));
        }

        queue_or_emit_start(
            &app_handle,
            &runtime_state,
            StartPayload { close_on_end: true },
        );
    });
}

fn spawn_random_mode(app_handle: AppHandle, runtime_state: Arc<RuntimeState>, cfg: CliConfig) {
    thread::spawn(move || {
        let mut rng = rand::thread_rng();
        let mut fired = 0u64;

        loop {
            let wait_ms = if cfg.random_min_ms == cfg.random_max_ms {
                cfg.random_min_ms
            } else {
                rng.gen_range(cfg.random_min_ms..=cfg.random_max_ms)
            };

            if wait_ms > 0 {
                thread::sleep(Duration::from_millis(wait_ms));
            }

            let is_last = cfg.random_count != 0 && fired + 1 >= cfg.random_count;
            queue_or_emit_start(
                &app_handle,
                &runtime_state,
                StartPayload {
                    close_on_end: is_last,
                },
            );

            fired += 1;
            if cfg.random_count != 0 && fired >= cfg.random_count {
                break;
            }
        }
    });
}

fn spawn_daemon_mode(app_handle: AppHandle, runtime_state: Arc<RuntimeState>, bind: String) {
    thread::spawn(move || {
        let server = match Server::http(&bind) {
            Ok(server) => server,
            Err(error) => {
                eprintln!("Failed to bind daemon server at {bind}: {error}");
                return;
            }
        };

        println!("Foxy daemon listening on http://{bind}");
        println!("open http://{bind}/foxy");

        for request in server.incoming_requests() {
            let is_foxy_get =
                request.method() == &Method::Get && request.url().starts_with("/foxy");
            let is_health_get = request.method() == &Method::Get && request.url() == "/health";

            let response = if is_foxy_get {
                queue_or_emit_start(
                    &app_handle,
                    &runtime_state,
                    StartPayload {
                        close_on_end: false,
                    },
                );
                Response::from_string("queued\n").with_status_code(StatusCode(202))
            } else if is_health_get {
                Response::from_string("ok\n").with_status_code(StatusCode(200))
            } else {
                Response::from_string("not found\n").with_status_code(StatusCode(404))
            };

            if let Err(error) = request.respond(response) {
                eprintln!("Failed to respond to daemon request: {error}");
            }
        }
    });
}

#[tauri::command]
fn frontend_ready(
    app_handle: AppHandle,
    runtime_state: State<'_, Arc<RuntimeState>>,
) -> Result<(), String> {
    runtime_state.frontend_ready.store(true, Ordering::SeqCst);

    let pending_payloads = {
        let mut pending = runtime_state
            .pending
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        std::mem::take(&mut *pending)
    };

    for payload in pending_payloads {
        app_handle
            .emit("foxy:start", payload)
            .map_err(|error| error.to_string())?;
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cli_config = parse_cli_config().unwrap_or_else(|error| {
        eprintln!("{error}\n\n{}", usage_text());
        std::process::exit(2);
    });

    let runtime_state = Arc::new(RuntimeState::default());
    let setup_runtime_state = runtime_state.clone();
    let setup_cli_config = cli_config.clone();

    tauri::Builder::default()
        .manage(runtime_state)
        .invoke_handler(tauri::generate_handler![frontend_ready])
        .setup(move |app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let app_handle = app.handle().clone();
            match setup_cli_config.mode {
                RunMode::Once => {
                    spawn_once_mode(
                        app_handle,
                        setup_runtime_state.clone(),
                        setup_cli_config.delay_ms,
                    );
                }
                RunMode::Random => {
                    spawn_random_mode(
                        app_handle,
                        setup_runtime_state.clone(),
                        setup_cli_config.clone(),
                    );
                }
                RunMode::Daemon => {
                    spawn_daemon_mode(
                        app_handle,
                        setup_runtime_state.clone(),
                        setup_cli_config.daemon_bind.clone(),
                    );
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
