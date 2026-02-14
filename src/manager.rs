use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, OnceLock};
pub struct Task {
    handle: JoinHandle<()>,
    cancel_token: CancellationToken,
}

pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

static GLOBAL_MANAGER: OnceLock<TaskManager> = OnceLock::new();

impl TaskManager {
    pub fn global() -> &'static Self {
        GLOBAL_MANAGER.get_or_init(|| Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn spawn<F, Fut>(id: &str, f: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let manager = Self::global();
        let token = CancellationToken::new();
        let id_str = id.to_string();

        let token_for_task = token.clone();
        let id_for_cleanup = id_str.clone();

        // Starts the task
        let handle = tokio::spawn(async move {
            f(token_for_task).await;
            // Remove task from list if it is finished
            Self::remove(&id_for_cleanup).await;
        });

        // Insert task to the list
        let mut tasks = manager.tasks.write().await;
        tasks.insert(id_str, Task { handle, cancel_token: token });
    }

    pub async fn remove(id: &str) {
        let manager = Self::global();
        manager.tasks.write().await.remove(id);
    }

    pub async fn stop_all() {
        let manager = Self::global();
        let tasks = manager.tasks.write().await;

        for (_, task) in tasks.iter() {
            task.cancel_token.cancel();
        }
    }

    pub async fn has_tasks() -> bool {
        let manager = Self::global();
        let tasks = manager.tasks.read().await;
        !tasks.is_empty()
    }
    
    pub async fn task_count() -> usize {
        let manager = Self::global();
        let tasks = manager.tasks.read().await;
        tasks.len()
    }
}