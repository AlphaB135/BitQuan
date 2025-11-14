mod commands;
use commands::WalletState;

fn main() {
    tauri::Builder::default()
        .manage(WalletState::default())
        .invoke_handler(tauri::generate_handler![
            commands::create_wallet,
            commands::unlock_wallet,
            commands::sign_transaction,
            commands::get_wallet_status,
            commands::lock_wallet,
            commands::clear_cache,
            commands::send_raw_transaction,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}