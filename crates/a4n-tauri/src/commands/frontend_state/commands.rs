use super::file::{frontend_state_lock, frontend_state_path, read_state_file, write_state_file};
use super::rules::{validate_key, validate_value};
use std::collections::BTreeMap;
use tauri::State;

#[tauri::command]
pub fn get_all_frontend_state(
    state: State<'_, super::FrontendStatePath>,
) -> Result<BTreeMap<String, String>, String> {
    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    Ok(read_state_file(&path)?.entries)
}

#[tauri::command]
pub fn set_frontend_state_many(
    state: State<'_, super::FrontendStatePath>,
    entries: BTreeMap<String, String>,
) -> Result<(), String> {
    for (key, value) in &entries {
        validate_key(key)?;
        validate_value(value)?;
    }

    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    let mut file = read_state_file(&path)?;
    for (key, value) in entries {
        file.entries.insert(key, value);
    }
    write_state_file(&path, &file)
}

#[tauri::command]
pub fn remove_frontend_state_many(
    state: State<'_, super::FrontendStatePath>,
    keys: Vec<String>,
) -> Result<(), String> {
    for key in &keys {
        validate_key(key)?;
    }

    let _guard = frontend_state_lock().lock();
    let path = frontend_state_path(&state);
    let mut file = read_state_file(&path)?;
    for key in keys {
        file.entries.remove(&key);
    }
    write_state_file(&path, &file)
}
