use std::sync::Arc;

use yap_daemon::{dbus, runtime::LocalRuntime, store::StateStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = Arc::new(StateStore::discover()?);
    let runtime = LocalRuntime::discover(Arc::clone(&store))?;
    let levels = runtime.subscribe_levels();
    let warming = Arc::clone(&runtime);
    tokio::spawn(async move {
        if let Err(error) = warming.warm().await {
            eprintln!("yapd: local speech model is not warm: {error}");
        }
    });
    dbus::serve(runtime, store, levels).await?;
    Ok(())
}
