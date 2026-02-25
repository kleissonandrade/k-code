use std::time::Duration;

use crossterm::event::{
    Event as CrosstermEvent, EventStream, KeyEvent, KeyEventKind, MouseEvent,
};
use futures::StreamExt;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum Event {
    Key(KeyEvent),
    Mouse(MouseEvent),
    Resize(u16, u16),
    Tick,
    FocusGained,
    FocusLost,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    _task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: Duration) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_interval = tokio::time::interval(tick_rate);

            loop {
                let event = tokio::select! {
                    _ = tick_interval.tick() => Event::Tick,
                    maybe_event = reader.next() => {
                        match maybe_event {
                            Some(Ok(evt)) => match evt {
                                CrosstermEvent::Key(key) => {
                                    if key.kind == KeyEventKind::Press {
                                        Event::Key(key)
                                    } else {
                                        continue;
                                    }
                                }
                                CrosstermEvent::Mouse(mouse) => Event::Mouse(mouse),
                                CrosstermEvent::Resize(w, h) => Event::Resize(w, h),
                                CrosstermEvent::FocusGained => Event::FocusGained,
                                CrosstermEvent::FocusLost => Event::FocusLost,
                                _ => continue,
                            },
                            Some(Err(_)) => continue,
                            None => break,
                        }
                    }
                };
                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Self { rx, _task: task }
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.rx.recv().await
    }
}
