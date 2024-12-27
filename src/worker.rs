use crate::config::ValidatedRequest;
use std::sync::Arc;

#[derive(Debug)]
pub struct Pool {
    bc: tokio::sync::broadcast::Sender<WorkerCommand>,
    handles: Vec<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
enum WorkerCommand {
    Request(ValidatedRequest),
    Shutdown,
}

impl Pool {
    pub fn new(num_workers: u32, base_url: reqwest::Url) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(num_workers as usize);
        let base_url = Arc::new(base_url);
        Self {
            handles: (0..num_workers)
                .map(|_| tokio::spawn(Self::worker(tx.subscribe(), Arc::clone(&base_url))))
                .collect(),
            bc: tx,
        }
    }

    #[must_use]
    pub fn submit_requests(&self, requests: Vec<ValidatedRequest>) -> Vec<ValidatedRequest> {
        let mut cant_send = vec![];
        for req in requests {
            let _ = self.bc.send(WorkerCommand::Request(req)).map_err(|err| {
                let WorkerCommand::Request(req) = err.0 else {
                    unreachable!()
                };
                cant_send.push(req);
            });
        }
        cant_send
    }

    async fn worker(
        mut rx: tokio::sync::broadcast::Receiver<WorkerCommand>,
        base_url: Arc<reqwest::Url>,
    ) {
        'worker: loop {
            match rx.recv().await {
                Ok(WorkerCommand::Request(req)) => {
                    let _ = req.send(base_url.as_ref()).await;
                    todo!()
                }
                Ok(WorkerCommand::Shutdown) => break 'worker,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break 'worker,
                _ => todo!(),
            }
        }
    }

    pub async fn shutdown(self) {
        let _ = self.bc.send(WorkerCommand::Shutdown);
        std::mem::drop(self.bc);
        for handle in self.handles {
            let _ = handle.await;
        }
    }
}
