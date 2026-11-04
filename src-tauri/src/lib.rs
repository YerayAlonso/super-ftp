mod commands;
mod error;
mod models;
mod paths;
mod ssh;
mod state;
mod storage;
mod transfer;
mod updates;

#[cfg(test)]
mod integration_tests;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let store = storage::Store::load(app.path().app_config_dir()?)?;
            app.manage(state::AppState::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_connections,
            commands::save_connection,
            commands::delete_connection,
            commands::connect_session,
            commands::trust_host_key,
            commands::disconnect_session,
            commands::list_remote,
            commands::list_local,
            commands::create_local_directory,
            commands::delete_local,
            commands::get_local_home,
            commands::create_remote_directory,
            commands::rename_remote,
            commands::delete_remote,
            commands::enqueue_transfers,
            commands::get_transfers,
            commands::cancel_transfer,
            commands::resolve_transfer_conflict,
            commands::clear_finished_transfers,
            updates::check_updates,
            updates::open_releases_page,
        ])
        .run(tauri::generate_context!())
        .expect("No s’ha pogut iniciar Super FTP");
}
