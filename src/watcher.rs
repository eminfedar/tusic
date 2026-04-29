use inotify::{EventMask, Inotify, WatchMask};
use std::{path::PathBuf, sync::mpsc::Sender};

use crate::msg::Message;

pub fn watch(paths: Vec<PathBuf>, sender: Sender<Message>) -> anyhow::Result<()> {
    // File Monitor Changes
    let mut inotify = Inotify::init()?;

    for p in paths {
        // watcher::watch_create_remove(&p, task_tx.clone())?;
        inotify.watches().add(
            &p,
            WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE | WatchMask::MOVE,
        )?;
    }

    std::thread::spawn(move || {
        let mut buffer = [0; 1024];
        loop {
            match inotify.read_events_blocking(&mut buffer) {
                Ok(events) => {
                    for e in events {
                        match e.mask {
                            EventMask::CREATE
                            | EventMask::DELETE
                            | EventMask::MOVED_TO
                            | EventMask::MOVED_FROM => {
                                let _ = sender.send(Message::FileChanged(e.to_owned()));
                            }
                            _ => (),
                        }
                    }
                }
                Err(_e) => (),
            }
        }
    });

    Ok(())
}
