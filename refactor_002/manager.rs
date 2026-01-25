use std::collections::HashMap;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tokio::sync::{ watch, RwLock };
use std::future::Future;
use std::sync::{ Arc, OnceLock };

pub struct Task {
    pub id: String,
    handle: JoinHandle<()>,
    cancel_token: CancellationToken,
    state_rx: watch::Receiver<TaskState>,
}

impl Task {
    pub fn state(&self) -> TaskState {
        self.state_rx.borrow().clone()
    }

    pub fn soft_stop(&self) {
        self.cancel_token.cancel();
    }

    pub fn force_stop(self) {
        self.handle.abort();
    }
}

#[derive(Clone)]
pub struct TaskState {
    // Progress in percent
    // Example: 25.3% and as bar [====------]
    pub progress_bar: f32,
    // Message or description from the current task
    pub message: String,
    // Something like "[current/max]"
    // For example [2/5]
    pub progress_max: usize,
    pub progress_current: usize,
}

static GLOBAL_MANAGER: OnceLock<TaskManager> = OnceLock::new();

pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskManager {
    // Global Manager instance
    pub fn global() -> &'static Self {
        GLOBAL_MANAGER.get_or_init(|| Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Creates a new dolphin... uhm no I meant it crates a new controllable async task
    pub async fn spawn<F, Fut>(id: &str, f: F)
    where
        F: FnOnce(CancellationToken, watch::Sender<TaskState>) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let manager = Self::global();
        let token = CancellationToken::new();
        let (tx, rx) = watch::channel(TaskState {
            progress_bar: 0.0,
            message: "Initializing...".to_string(),
            progress_max: 0,
            progress_current: 0,
        });

        let id_str = id.to_string();
        let id_for_task = id_str.clone();
        let token_for_task = token.clone();

        let handle = tokio::spawn(async move {
            f(token_for_task, tx).await;
            Self::remove(&id_for_task).await;
        });

        let task = Task {
            id: id_str.clone(),
            handle,
            cancel_token: token,
            state_rx: rx,
        };

        manager.tasks.write().await.insert(id_str, task);
    }

    // Stopping methods

    /// Stops all current tasks sweetly
    pub async fn stop_all() {
        let manager = Self::global();
        let mut handles = Vec::new();

        {
            let mut tasks = manager.tasks.write().await;
            for (_, task) in tasks.drain() {
                task.soft_stop();
                handles.push(task.handle);
            }
        }

        for h in handles {
            let _ = h.await;
        }
    }

    /// STOP ALL TASKS NOW
    pub async fn force_stop_all() {
        let manager = Self::global();
        let mut tasks = manager.tasks.write().await;
        for (_, task) in tasks.drain() {
            task.handle.abort(); // doing the force stop
        }
    }

    // Getter/Setter methods for the map

    /// Is currently tasks running?
    /// Returns a Boolean
    pub async fn has_tasks() -> bool {
        let manager = Self::global();
        !manager.tasks.read().await.is_empty()
    }

    pub async fn list_all() -> Vec<(String, TaskState)> {
        let manager = Self::global();
        let tasks = manager.tasks.read().await;
        tasks.iter().map(|(id, t)| (id.clone(), t.state())).collect()
    }

    pub async fn remove(id: &str) {
        let manager = Self::global();
        manager.tasks.write().await.remove(id);
    }
}