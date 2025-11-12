#[tauri::command]
async fn test_connection() -> Result<String, String> {
    let client = reqwest::Client::new();
    let rpc_url = "http://localhost:8332";
    
    // Test basic connectivity
    match client.get(rpc_url).send().await {
        Ok(response) => Ok("Basic connectivity works".to_string()),
        Err(e) => Err(format!("Basic connectivity failed: {}", e)),
    }
}