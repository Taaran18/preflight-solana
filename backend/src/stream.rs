use std::{convert::Infallible, future::Future};

use axum::{
    body::Body,
    http::header,
    response::Response,
};
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

use crate::{
    error::{AppResult, redact},
    types::{StageDef, StageStatus, StreamEvent},
};

#[derive(Clone)]
pub struct Emitter {
    tx: mpsc::Sender<Bytes>,
}

impl Emitter {
    pub async fn send(&self, event: StreamEvent) {
        if let Ok(mut line) = serde_json::to_vec(&event) {
            line.push(b'\n');
            let _ = self.tx.send(Bytes::from(line)).await;
        }
    }

    pub async fn stage(&self, id: &str, status: StageStatus, note: Option<String>) {
        self.send(StreamEvent::Stage {
            id: id.to_string(),
            status,
            note,
        })
        .await;
    }

    pub async fn stages(&self, stages: &[(&str, &str)]) {
        self.send(StreamEvent::Stages {
            stages: stages
                .iter()
                .map(|(id, label)| StageDef {
                    id: id.to_string(),
                    label: label.to_string(),
                })
                .collect(),
        })
        .await;
    }

    pub async fn notice(&self, text: impl Into<String>) {
        self.send(StreamEvent::Notice {
            text: redact(&text.into()),
        })
        .await;
    }

    pub fn closed(&self) -> bool {
        self.tx.is_closed()
    }
}

pub fn ndjson<F, Fut>(job: F) -> Response
where
    F: FnOnce(Emitter) -> Fut + Send + 'static,
    Fut: Future<Output = AppResult<()>> + Send + 'static,
{
    let (tx, rx) = mpsc::channel::<Bytes>(64);
    let emitter = Emitter { tx };
    tokio::spawn(async move {
        let tail = emitter.clone();
        if let Err(err) = job(emitter).await {
            if !err.is_user() {
                eprintln!("request failed: {}", err.message());
            }
            tail.send(StreamEvent::Error { message: err.message() }).await;
        }
        tail.send(StreamEvent::Done).await;
    });
    let stream = ReceiverStream::new(rx).map(Ok::<Bytes, Infallible>);
    Response::builder()
        .header(header::CONTENT_TYPE, "application/x-ndjson; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache, no-transform")
        .header("x-accel-buffering", "no")
        .body(Body::from_stream(stream))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}
