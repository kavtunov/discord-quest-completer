// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use once_cell::sync::OnceCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{path::BaseDirectory, AppHandle, Emitter, Listener, Manager};

mod rpc;
mod runner;

// Global static instance of the Discord client
static DISCORD_CLIENT: OnceCell<Mutex<Option<rpc::Client>>> = OnceCell::new();

fn get_discord_client() -> &'static Mutex<Option<rpc::Client>> {
    DISCORD_CLIENT.get_or_init(|| Mutex::new(None))
}

fn runner_resource_name() -> &'static str {
    #[cfg(target_os = "windows")]
    let runner_name = "data/src-win.exe";

    #[cfg(target_os = "linux")]
    let runner_name = "data/src-linux";

    #[cfg(target_os = "macos")]
    let runner_name = "data/src-darwin";

    runner_name
}

fn is_app_bundle(executable_name: &str) -> bool {
    cfg!(target_os = "macos") && executable_name.ends_with(".app")
}

fn bundle_binary_name(app_name: &str) -> String {
    app_name
        .strip_suffix(".app")
        .unwrap_or(app_name)
        .to_string()
}

fn game_folder_path(exe_dir: &Path, path: &str, app_id: &str) -> PathBuf {
    // Discord detectable paths often use Windows separators; normalize for Linux/macOS.
    let normalized = path.replace('\\', "/");
    let mut folder = exe_dir.join("games").join(app_id);

    for segment in normalized.split('/').filter(|segment| !segment.is_empty()) {
        folder = folder.join(segment);
    }

    folder
}

fn app_exe_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn runner_file_name() -> &'static str {
    Path::new(runner_resource_name())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("src-linux")
}

fn resolve_runner_template(handle: &AppHandle) -> Result<PathBuf, String> {
    let resource_name = runner_resource_name();
    let file_name = runner_file_name();
    let exe_dir = app_exe_dir();

    let mut candidates: Vec<PathBuf> = Vec::new();

    // Prefer files next to the real binary (portable dist-linux/)
    candidates.push(exe_dir.join("data").join(file_name));
    candidates.push(exe_dir.join("resources").join(file_name));
    candidates.push(exe_dir.join(file_name));
    candidates.push(PathBuf::from("data").join(file_name));
    candidates.push(PathBuf::from(resource_name));

    if let Ok(path) = handle
        .path()
        .resolve(resource_name, BaseDirectory::Resource)
    {
        candidates.push(path);
    }
    if let Ok(path) = handle.path().resolve(file_name, BaseDirectory::Resource) {
        candidates.push(path);
    }

    for candidate in &candidates {
        if candidate.exists() {
            println!("Using runner template: {:?}", candidate);
            return Ok(candidate.clone());
        }
    }

    Err(format!(
        "Runner template not found. Expected `{}` next to the app (e.g. dist-linux/data/{}). Tried: {:?}",
        resource_name,
        file_name,
        candidates
    ))
}

/// Make sure portable `data/<runner>` exists beside the binary so Play never depends on /usr/lib.
fn ensure_portable_runner(handle: &AppHandle) -> Result<PathBuf, String> {
    let exe_dir = app_exe_dir();
    let portable = exe_dir.join("data").join(runner_file_name());
    if portable.exists() {
        return Ok(portable);
    }

    let template = resolve_runner_template(handle)?;
    if template == portable {
        return Ok(portable);
    }

    fs::create_dir_all(portable.parent().unwrap_or(exe_dir.as_path()))
        .map_err(|e| format!("Failed to create data/: {e}"))?;
    fs::copy(&template, &portable).map_err(|e| {
        format!(
            "Failed to copy runner template {:?} -> {:?}: {e}",
            template, portable
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&portable)
            .map_err(|e| format!("Failed to read runner metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&portable, perms)
            .map_err(|e| format!("Failed to chmod runner: {e}"))?;
    }

    println!("Installed portable runner at {:?}", portable);
    Ok(portable)
}

fn install_dummy_executable(
    handle: &AppHandle,
    path: &str,
    executable_name: &str,
    app_id: &str,
    display_name: Option<String>,
) -> Result<PathBuf, String> {
    let exe_dir = app_exe_dir();
    let game_folder = game_folder_path(&exe_dir, path, app_id);

    fs::create_dir_all(&game_folder)
        .map_err(|e| format!("Failed to create game folder {:?}: {e}", game_folder))?;

    let resource_path = ensure_portable_runner(handle)?;
    println!(
        "Installing dummy {:?} from {:?}",
        executable_name, resource_path
    );

    if is_app_bundle(executable_name) {
        let app_bundle_path = game_folder.join(executable_name);
        let bundle_display_name = display_name
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| bundle_binary_name(executable_name));
        return make_macos_app_bundle(
            &app_bundle_path,
            executable_name,
            &bundle_display_name,
            &resource_path,
        );
    }

    let target = game_folder.join(executable_name);
    fs::copy(&resource_path, &target).map_err(|e| {
        format!(
            "Failed to copy dummy executable {:?} -> {:?}: {e}",
            resource_path, target
        )
    })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&target)
            .map_err(|e| format!("Failed to get file metadata: {e}"))?
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&target, perms)
            .map_err(|e| format!("Failed to set executable permissions: {e}"))?;
    }

    if !target.exists() {
        return Err(format!("Dummy executable missing after copy: {:?}", target));
    }

    Ok(target)
}

fn spawn_dummy_game(name: &str, executable_path: &Path, current_dir: &Path) -> Result<(), String> {
    if !executable_path.exists() {
        return Err(format!(
            "Cannot launch missing executable: {:?}",
            executable_path
        ));
    }

    let mut cmd = std::process::Command::new(executable_path);
    cmd.args(["--title", name]).current_dir(current_dir);

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Keep a stable process name for Discord (/proc/self/comm uses the file name).
        cmd.process_group(0);
    }

    cmd.spawn()
        .map_err(|e| format!("Failed to start {:?}: {e}", executable_path))?;
    Ok(())
}

fn make_macos_app_bundle(
    app_bundle_path: &Path,
    app_name: &str,
    display_name: &str,
    runner_template: &Path,
) -> Result<PathBuf, String> {
    let binary_name = bundle_binary_name(app_name);
    let macos_dir = app_bundle_path.join("Contents/MacOS");
    fs::create_dir_all(&macos_dir)
        .map_err(|e| format!("Failed to create app bundle directories: {}", e))?;

    let target_executable_path = macos_dir.join(&binary_name);
    fs::copy(runner_template, &target_executable_path)
        .map_err(|e| format!("Failed to copy dummy executable: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(&target_executable_path)
            .map_err(|e| format!("Failed to read executable permissions: {}", e))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&target_executable_path, permissions)
            .map_err(|e| format!("Failed to set executable permissions: {}", e))?;
    }

    let info_plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDisplayName</key>
    <string>{display_name}</string>
    <key>CFBundleExecutable</key>
    <string>{binary_name}</string>
    <key>CFBundleIdentifier</key>
    <string>me.markterence.discordquestcompleter.dummy</string>
    <key>CFBundleName</key>
    <string>{display_name}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
</dict>
</plist>
"#
    );

    fs::write(app_bundle_path.join("Contents/Info.plist"), info_plist)
        .map_err(|e| format!("Failed to write Info.plist: {}", e))?;

    Ok(target_executable_path)
}

fn launch_executable_path(game_folder_path: &Path, executable_name: &str) -> PathBuf {
    if is_app_bundle(executable_name) {
        game_folder_path
            .join(executable_name)
            .join("Contents/MacOS")
            .join(bundle_binary_name(executable_name))
    } else {
        game_folder_path.join(executable_name)
    }
}

fn stop_process_name(exec_name: &str) -> String {
    if is_app_bundle(exec_name) {
        bundle_binary_name(exec_name)
    } else {
        exec_name.to_string()
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
async fn create_fake_game(
    handle: tauri::AppHandle,
    path: &str,
    executable_name: &str,
    path_len: i64,
    app_id: String,
    display_name: Option<String>,
) -> Result<String, String> {
    let _ = path_len;
    let target =
        install_dummy_executable(&handle, path, executable_name, &app_id, display_name)?;
    Ok(format!("Dummy executable ready at: {:?}", target))
}

#[cfg(target_os = "macos")]
fn launch_macos_app_bundle(app_bundle_path: &Path, title: &str) -> Result<(), String> {
    let mut command = std::process::Command::new("open");
    command
        .arg("-n")
        .arg("-g")
        .arg("-a")
        .arg(app_bundle_path)
        .arg("--args")
        .arg("--title")
        .arg(title);

    command
        .spawn()
        .map_err(|e| format!("Failed to launch app bundle with open: {}", e))?;

    Ok(())
}

#[tauri::command(rename_all = "snake_case")]
async fn run_background_process(
    handle: tauri::AppHandle,
    name: &str,
    path: &str,
    executable_name: &str,
    path_len: i64,
    app_id: String,
) -> Result<String, String> {
    let _ = path_len;
    let exe_dir = app_exe_dir();
    let game_folder = game_folder_path(&exe_dir, path, &app_id);

    if is_app_bundle(executable_name) {
        #[cfg(target_os = "macos")]
        {
            let target = install_dummy_executable(
                &handle,
                path,
                executable_name,
                &app_id,
                Some(name.to_string()),
            )?;
            let bundle_path = target
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| game_folder.join(executable_name));
            launch_macos_app_bundle(&bundle_path, name)?;
            return Ok("App bundle launched successfully".to_string());
        }

        #[cfg(not(target_os = "macos"))]
        {
            let _ = (handle, name);
            return Err("App bundle launches are only supported on macOS".to_string());
        }
    }

    // Always (re)install dummy before launch so Play works even after failed earlier attempts.
    let executable_path = install_dummy_executable(
        &handle,
        path,
        executable_name,
        &app_id,
        Some(name.to_string()),
    )?;

    spawn_dummy_game(name, &executable_path, &game_folder)?;
    Ok(format!("Process started: {:?}", executable_path))
}

/// One-shot Play for quests: install dummy + launch. Prefer this from the UI.
#[tauri::command(rename_all = "snake_case")]
async fn play_quest_game(
    handle: tauri::AppHandle,
    name: &str,
    path: &str,
    executable_name: &str,
    app_id: String,
) -> Result<String, String> {
    run_background_process(
        handle,
        name,
        path,
        executable_name,
        0,
        app_id,
    )
    .await
}

#[tauri::command(rename_all = "snake_case")]
async fn stop_process(exec_name: String) -> Result<(), String> {
    let process_name = stop_process_name(&exec_name);

    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("taskkill")
            .arg("/F")
            .arg("/IM")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute taskkill: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to stop process: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    #[cfg(target_os = "macos")]
    {
        if is_app_bundle(&exec_name) {
            let bundle_pattern = format!("{}/Contents/MacOS", exec_name);
            let output = std::process::Command::new("pkill")
                .arg("-f")
                .arg(&bundle_pattern)
                .output()
                .map_err(|e| format!("Failed to execute pkill: {}", e))?;

            if output.status.success() {
                return Ok(());
            }
        }

        let output = std::process::Command::new("pkill")
            .arg("-x")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute pkill: {}", e))?;

        if output.status.success() || output.status.code() == Some(1) {
            Ok(())
        } else {
            Err(format!(
                "Failed to stop process: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
    #[cfg(target_os = "linux")]
    {
        // Prefer exact process name match; fall back to full cmdline match for long names.
        let exact = std::process::Command::new("pkill")
            .arg("-x")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute pkill: {}", e))?;

        if exact.status.success() || exact.status.code() == Some(1) {
            if exact.status.success() {
                return Ok(());
            }
        }

        let output = std::process::Command::new("pkill")
            .arg("-f")
            .arg(&process_name)
            .output()
            .map_err(|e| format!("Failed to execute pkill: {}", e))?;

        if output.status.success() || output.status.code() == Some(1) {
            // pkill returns 1 if no processes were killed, which is fine
            Ok(())
        } else {
            Err(format!(
                "Failed to stop process: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }
}

/// Usage: Calling from JS:
/// ```javascript
/// await invoke('connect_to_discord_rpc_3', json, 'connect' | 'disconnect');
#[tauri::command(rename_all = "snake_case")]
fn connect_to_discord_rpc_3(handle: AppHandle, activity_json: String, action: String) {
    let app = handle.clone();

    let event_connecting = "client_connecting";
    let event_connected = "client_connected";
    let event_disconnect = "event_disconnect";
    let event_connect = "event_connect";

    let activity = runner::parse_activity_json(&activity_json).unwrap();

    let connecting_payload = serde_json::json!({
        "app_id": activity.app_id,
    });

    let client_option = {
        let mut client_guard = get_discord_client().lock().unwrap();
        // Take the client out, leaving None in its place
        client_guard.take()
        // MutexGuard is dropped here at the end of scope
    };

    let task = tauri::async_runtime::spawn(async move {
        handle
            .emit(event_connecting, connecting_payload)
            .unwrap_or_else(|e| eprintln!("Failed to emit event: {}", e));

        let client = runner::set_activity(activity_json)
            .await
            .map_err(|e| {
                println!("Failed to set activity: {}", e);
            })
            .unwrap();

        let connected_payload = serde_json::json!({
            "app_id": activity.app_id,
        });

        {
            let mut client_guard = get_discord_client().lock().unwrap();
            *client_guard = Some(client);
        }

        handle
            .emit(event_connected, connected_payload)
            .unwrap_or_else(|e| {
                eprintln!("Failed to emit event: {}", e);
            });

        handle.listen(event_disconnect, move |_| {
            println!("Disconnecting from Discord RPC inner");
            let disconnect_task = tauri::async_runtime::spawn(async move {
                let client_option = {
                    let mut client_guard = get_discord_client().lock().unwrap();
                    // Take the client out, leaving None in its place
                    client_guard.take()
                    // MutexGuard is dropped here at the end of scope
                };
                if let Some(client) = client_option {
                    client.discord.disconnect().await;
                    println!("Disconnected from Discord RPC inner");
                }
            });
            // disconnect_task.abort();
        });
    });

    app.listen(event_disconnect, move |_| {
        println!("Disconnecting from Discord RPC...");
        task.abort();
    });
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_gh_mirror() -> Result<serde_json::Value, String> {
    let res = tauri_plugin_http::reqwest::get(
        "https://markterence.github.io/discord-quest-completer/detectable.json",
    )
    .await
    .map_err(|e| format!("GitHub mirror request failed: {e}"))?;
    let text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read GitHub mirror body: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("Invalid GitHub mirror JSON: {e}"))
}

#[tauri::command(rename_all = "snake_case")]
async fn fetch_gamelist_from_discord() -> Result<serde_json::Value, String> {
    let res = tauri_plugin_http::reqwest::get("https://discord.com/api/applications/detectable")
        .await
        .map_err(|e| format!("Discord detectable request failed: {e}"))?;
    let text = res
        .text()
        .await
        .map_err(|e| format!("Failed to read Discord body: {e}"))?;
    serde_json::from_str(&text).map_err(|e| format!("Invalid Discord JSON: {e}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            if let Err(e) = ensure_portable_runner(app.handle()) {
                eprintln!("Warning: could not prepare portable runner: {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            create_fake_game,
            stop_process,
            connect_to_discord_rpc_3,
            run_background_process,
            play_quest_game,
            fetch_gamelist_gh_mirror,
            fetch_gamelist_from_discord
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}