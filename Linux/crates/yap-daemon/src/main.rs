use std::sync::Arc;

use yap_daemon::{dbus, runtime::LocalRuntime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = LocalRuntime::discover()?;
    let warming = Arc::clone(&runtime);
    tokio::spawn(async move {
        if let Err(error) = warming.warm().await {
            eprintln!("yapd: local speech model is not warm: {error}");
        }
    });
    dbus::serve(runtime).await?;
    Ok(())
}
