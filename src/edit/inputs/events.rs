use std::{thread, time::Duration};

use crossbeam::channel::{unbounded, Receiver, RecvError, Sender};
use crossterm::event;

use super::{key::Key, InputEvent};

pub struct Events {
    rx: Receiver<InputEvent>,
    _tx: Sender<InputEvent>,
}

impl Events {
    pub fn new(tick_rate: Duration) -> Events {
        let (tx, rx) = unbounded();

        let event_tx = tx.clone();
        thread::spawn(move || loop {
            if crossterm::event::poll(tick_rate).unwrap() {
                if let event::Event::Key(key) = event::read().unwrap() {
                    let key = Key::from(key);
                    event_tx.send(InputEvent::Input(key)).unwrap();
                }
            }

            event_tx.send(InputEvent::Tick).unwrap()
        });

        Events { rx, _tx: tx }
    }

    pub fn next(&self) -> Result<InputEvent, RecvError> {
        self.rx.recv()
    }
}
