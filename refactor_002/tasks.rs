use crate::manager;
pub async fn init_tasks() {
    spawn_scanner().await;
}

async fn spawn_scanner() {
    manager::TaskManager::spawn("Crawler", |cancel_token, tx| async move {
        let _ = tx.send(manager::TaskState {
            progress_bar: 0.0,
            message: "Starting crawl worker...".to_string(),
            progress_max: 0,
            progress_current: 0
        });
        // TODO
        // spawn_crawler().await;
    }).await;
}

async fn spawn_crawler() {
    manager::TaskManager::spawn("Scan", |cancel_token, tx| async move {
        let _ = tx.send(manager::TaskState {
            progress_bar: 0.0,
            message: "Scanning server...".to_string(),
            progress_max: 100,
            progress_current: 0
        });
        // TODO
    }).await;
}