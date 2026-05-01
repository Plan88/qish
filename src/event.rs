use std::sync::mpsc;
use std::thread;

use crossterm::event::{self, Event, KeyEvent, KeyEventKind};

use crate::error::AppError;

pub enum AppEvent {
    Key(KeyEvent),
}

pub struct EventReader {
    rx: mpsc::Receiver<AppEvent>,
}

impl EventReader {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            loop {
                match event::read() {
                    Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                        if tx.send(AppEvent::Key(key)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        });

        Self { rx }
    }

    pub fn next(&self) -> Result<AppEvent, AppError> {
        self.rx
            .recv()
            .map_err(|_| AppError::Io(std::io::Error::other("event channel closed")))
    }
}
