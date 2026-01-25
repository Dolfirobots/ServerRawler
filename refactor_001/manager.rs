use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;
use tokio::task::JoinSet;
use std::time::Duration;
use crate::logger_OLD::Logger;


pub struct TaskManager {
    cancellation_token: CancellationToken,
    total_tasks: Arc<AtomicUsize>,
    active_tasks: Arc<AtomicUsize>,
    join_set: Arc<Mutex<JoinSet<()>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
            total_tasks: Arc::new(AtomicUsize::new(0)),
            active_tasks: Arc::new(AtomicUsize::new(0)),
            join_set: Arc::new(Mutex::new(JoinSet::new())),
        }
    }

    pub async fn spawn_task<F, Fut>(&self, task_logic: F)
    where
        F: FnOnce(CancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.total_tasks.fetch_add(1, Ordering::SeqCst);
        self.active_tasks.fetch_add(1, Ordering::SeqCst);

        let token = self.cancellation_token.clone();
        let active_counter = self.active_tasks.clone();

        let mut set = self.join_set.lock().await;
        set.spawn(async move {
            task_logic(token).await;

            active_counter.fetch_sub(1, Ordering::SeqCst);
        });
    }

    pub async fn wait_for_shutdown(&self, logger: &Logger) {
        let mut shutdown_triggered = false;

        loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    if shutdown_triggered {
                        logger.critical("Force shutdown! Data can be corrupted.").send().await;
                        std::process::exit(1);
                    } else {
                        shutdown_triggered = true;
                        logger.warning("Shutting down all tasks... (Ctrl+C for force shutdown)").send().await;

                        self.cancellation_token.cancel();
                        self.monitor_progress(logger).await;
                    }
                }
            }
        }
    }

    async fn monitor_progress(&self, logger: &Logger) {
        loop {
            let active = self.active_tasks.load(Ordering::SeqCst);
            let total = self.total_tasks.load(Ordering::SeqCst);

            if active == 0 {
                logger.success("All tasks were successfully shutdown. Bye!").send().await;
                std::process::exit(0);
            }

            let done = total - active;
            let percent = if total > 0 { (done as f64 / total as f64) * 100.0 } else { 100.0 };

            let msg = format!("Waiting auf tasks: {}/{} finished ({:.1}%)", done, total, percent);
            logger.info(&msg).send().await;

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}