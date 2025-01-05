use crate::config::ValidatedRequest;
use std::{num::NonZeroUsize, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct RequestResult {
    duration: std::time::Duration,
    result: Result<reqwest::Response, reqwest::Error>,
}

impl RequestResult {
    #[allow(dead_code)]
    pub fn result(&self) -> Result<&reqwest::Response, &reqwest::Error> {
        self.result.as_ref()
    }

    pub fn duration(&self) -> std::time::Duration {
        self.duration
    }
}

#[derive(Debug, Clone)]
pub enum Command {
    Request(ValidatedRequest),
    Shutdown,
}

#[derive(Debug)]
pub struct Pool {
    queue: Arc<Mutex<Vec<Command>>>,
    notify: Arc<tokio::sync::Notify>,
    handles: Vec<tokio::task::JoinHandle<()>>,
    response_rx: tokio::sync::mpsc::Receiver<RequestResult>,
}

impl Pool {
    pub fn new(num_workers: NonZeroUsize, base_url: &reqwest::Url) -> Self {
        let queue = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(tokio::sync::Notify::new());
        let (tx, rx) = tokio::sync::mpsc::channel(num_workers.get());
        let tx = Arc::new(tx);
        let handles = (0..num_workers.get())
            .map(|id| {
                tokio::spawn(Self::worker(
                    id,
                    Arc::clone(&queue),
                    Arc::clone(&notify),
                    Arc::clone(&tx),
                    base_url.clone(),
                ))
            })
            .collect::<Vec<_>>();
        Self {
            queue,
            notify,
            handles,
            response_rx: rx,
        }
    }

    async fn worker(
        _id: usize,
        queue: Arc<Mutex<Vec<Command>>>,
        notify: Arc<tokio::sync::Notify>,
        res_tx: Arc<tokio::sync::mpsc::Sender<RequestResult>>,
        base_url: reqwest::Url,
    ) {
        'worker: loop {
            let mut queue_lck = queue.lock().await;
            let cmd = 'cmd: loop {
                let cmd = queue_lck.pop();
                if let Some(cmd) = cmd {
                    drop(queue_lck);
                    break 'cmd cmd;
                }
                drop(queue_lck);
                notify.notified().await;
                queue_lck = queue.lock().await;
            };
            match cmd {
                Command::Request(req) => {
                    let start = std::time::Instant::now();
                    let result = req.send(&base_url).await;
                    let duration = start.elapsed();
                    if res_tx
                        .send(RequestResult { duration, result })
                        .await
                        .is_err()
                    {
                        break 'worker;
                    }
                }
                Command::Shutdown => {
                    let mut guard = queue.lock().await;
                    guard.push(Command::Shutdown);
                    drop(guard);
                    notify.notify_one();
                    break 'worker;
                }
            }
        }
    }

    pub async fn send_command(&self, cmd: Command) {
        let mut guard = self.queue.lock().await;
        guard.push(cmd);
        drop(guard);
        self.notify.notify_one();
    }

    pub async fn get_response(&mut self) -> Option<RequestResult> {
        self.response_rx.recv().await
    }

    #[allow(dead_code)]
    pub async fn shutdown(&mut self) -> Result<(), tokio::task::JoinError> {
        self.send_command(Command::Shutdown).await;
        for handle in std::mem::take(&mut self.handles) {
            handle.await?;
        }
        Ok(())
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        if !self.handles.is_empty() {
            let handles = std::mem::take(&mut self.handles);
            let queue = std::mem::take(&mut self.queue);
            tokio::spawn(async move {
                let mut guard = queue.lock().await;
                guard.push(Command::Shutdown);
                drop(guard);
                for handle in handles {
                    let _ = handle.await;
                }
            });
        }
    }
}

impl std::fmt::Display for RequestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.result {
            Ok(response) => f.write_fmt(format_args!(
                "(Elapsed: {}ms) {{ status: {} }}",
                self.duration.as_millis(),
                response.status(),
            )),
            Err(err) => f.write_fmt(format_args!(
                "(Elapsed: {}ms): {err:?}",
                self.duration.as_millis(),
            )),
        }
    }
}
