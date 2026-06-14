//! Writes `tracing` events to dated files under `logs/`

use chrono::Utc;
use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub struct FileLogLayer;

pub fn file_log_layer() -> FileLogLayer {
    FileLogLayer
}

#[derive(Default)]
struct MessageVisitor {
    message: Option<String>,
    fields: Vec<(String, String)>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        if field.name() == "message" {
            self.message = Some(value);
        } else {
            self.fields.push((field.name().to_string(), value));
        }
    }
}

impl MessageVisitor {
    fn into_text(self) -> String {
        let mut text = self.message.unwrap_or_default();
        for (name, value) in self.fields {
            if !text.is_empty() {
                text.push(' ');
            }
            let _ = write!(text, "{name}={value}");
        }
        text
    }
}

impl<S: Subscriber> Layer<S> for FileLogLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let text = visitor.into_text();

        let now = Utc::now();
        let dir = format!("logs/{}", now.format("%Y-%m-%d"));
        let timestamp = now.format("%Y-%m-%d %H:%M:%S");

        if fs::create_dir_all(&dir).is_err() {
            return;
        }

        if *event.metadata().level() == Level::ERROR {
            let meta = event.metadata();
            let location = match meta.file().zip(meta.line()) {
                Some((file, line)) => format!("{file}:{line}"),
                None => meta.target().to_string(),
            };
            let block = format!(
                "---------------------------------------\n\
                 {timestamp}\n\
                 Target: {target}\n\
                 Location: {location}\n\
                 Message: {text}\n\
                 ---------------------------------------\n",
                target = meta.target(),
            );
            append(&format!("{dir}/errors.log"), &block);
        } else {
            append(
                &format!("{dir}/info.log"),
                &format!("{timestamp} -- {text}\n"),
            );
        }
    }
}

fn append(path: &str, content: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(content.as_bytes());
    }
}
