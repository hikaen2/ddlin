#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use ddlin_core::{Dict, Dir, Hit, Library, Mode, Pos, Segment};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Default)]
struct AppState {
    root: RwLock<Option<PathBuf>>,
    lib: RwLock<Option<Arc<Library>>>,
}

#[derive(Serialize, Deserialize, Default)]
struct Config {
    root: Option<PathBuf>,
    #[serde(default)]
    fonts: Fonts,
}

/// Fonts for the UI, the headword list and the text pane. An empty family
/// means the system default; a size of 0 means the default size.
#[derive(Serialize, Deserialize, Clone)]
struct Fonts {
    #[serde(default)]
    ui: Font,
    list: Font,
    text: Font,
    #[serde(default = "yes")]
    antialias: bool,
}

impl Default for Fonts {
    fn default() -> Self {
        Fonts { ui: Font::default(), list: Font::default(), text: Font::default(), antialias: true }
    }
}

fn yes() -> bool {
    true
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct Font {
    family: String,
    size: u32,
}

#[derive(Serialize)]
struct Overview {
    root: Option<String>,
    dicts: Vec<Dict>,
}

fn config_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("config.json"))
}

fn load_config(app: &AppHandle) -> Config {
    config_path(app)
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let path = config_path(app).ok_or("no config directory")?;
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(path, serde_json::to_vec_pretty(config).unwrap()).map_err(|e| e.to_string())
}

fn open_root(state: &AppState, root: Option<PathBuf>) {
    let lib = root.as_deref().map(|r| Arc::new(Library::open(r)));
    *state.root.write().unwrap() = root;
    *state.lib.write().unwrap() = lib;
}

fn overview(state: &AppState) -> Overview {
    Overview {
        root: state.root.read().unwrap().as_ref().map(|p| p.display().to_string()),
        dicts: state.lib.read().unwrap().as_ref().map(|l| l.dicts().to_vec()).unwrap_or_default(),
    }
}

fn lib(state: &AppState) -> Result<Arc<Library>, String> {
    state.lib.read().unwrap().clone().ok_or_else(|| "No dictionary folder".into())
}

#[tauri::command]
async fn get_overview(state: State<'_, AppState>) -> Result<Overview, String> {
    Ok(overview(&state))
}

#[tauri::command]
async fn choose_folder(app: AppHandle, state: State<'_, AppState>) -> Result<Overview, String> {
    if let Some(dir) = app.dialog().file().set_title("Choose dictionary folder").blocking_pick_folder() {
        let root = dir.into_path().map_err(|e| e.to_string())?;
        save_config(&app, &Config { root: Some(root.clone()), ..load_config(&app) })?;
        open_root(&state, Some(root));
    }
    Ok(overview(&state))
}

#[tauri::command]
async fn get_fonts(app: AppHandle) -> Result<Fonts, String> {
    Ok(load_config(&app).fonts)
}

#[tauri::command]
async fn set_fonts(app: AppHandle, fonts: Fonts) -> Result<(), String> {
    save_config(&app, &Config { fonts, ..load_config(&app) })
}

/// Font families installed on the system, sorted.
#[tauri::command]
async fn font_families() -> Result<Vec<String>, String> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut names: Vec<String> =
        db.faces().filter_map(|f| f.families.first().map(|(name, _)| name.clone())).collect();
    names.sort();
    names.dedup();
    Ok(names)
}

#[tauri::command]
async fn search(state: State<'_, AppState>, id: usize, query: String, mode: Mode) -> Result<Vec<Hit>, String> {
    lib(&state)?.search(id, &query, mode).map_err(|e| e.to_string())
}

#[tauri::command]
async fn read_segment(state: State<'_, AppState>, id: usize, pos: Pos, dir: Dir) -> Result<Segment, String> {
    lib(&state)?.read_segment(id, pos, dir).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .setup(|app| {
            let config = load_config(app.handle());
            open_root(&app.state::<AppState>(), config.root);
            #[cfg(target_os = "linux")]
            app.get_webview_window("main").unwrap().with_webview(|webview| {
                use webkit2gtk::{SettingsExt, WebViewExt};
                if let Some(settings) = webview.inner().settings() {
                    settings.set_enable_smooth_scrolling(false);
                }
            })?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_overview, choose_folder, get_fonts, set_fonts, font_families, search, read_segment])
        .run(tauri::generate_context!())
        .expect("error while running ddlin");
}
