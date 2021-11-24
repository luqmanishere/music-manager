use std::sync::Arc;

use eyre::Result;
use log::error;

use crate::edit::app::App;

use super::IoEvent;

pub struct IoAsyncHandler {
    app: Arc<tokio::sync::Mutex<App>>,
}

impl IoAsyncHandler {
    pub fn new(app: Arc<tokio::sync::Mutex<App>>) -> Self {
        Self { app }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        let result = match io_event {
            IoEvent::Initialize => self.do_initialize().await,
        };

        if let Err(err) = result {
            error!(target: "IoEvent","{}", err);
        }

        let mut app = self.app.lock().await;
        app.loaded();
    }

    async fn do_initialize(&mut self) -> Result<()> {
        let mut app = self.app.lock().await;
        app.initialized();
        Ok(())
    }
}
