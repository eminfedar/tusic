use std::{path::PathBuf, sync::mpsc::Sender};

use anyhow::Result;
use notify::{
    event::{EventKind, ModifyKind},
    RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcher,
};

use crate::msg::Message;

/// Watches the music directories for changes using the cross-platform `notify`
/// crate (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on
/// Windows). When a relevant file event fires it sends a
/// [`Message::FileChanged`] so the app can re-scan the library.
pub struct Watcher {
    watcher: RecommendedWatcher,
    watched_paths: Vec<PathBuf>,
}

impl Watcher {
    pub fn new(paths: Vec<PathBuf>, tx: Sender<Message>) -> Result<Self> {
        // The callback runs on notify's own backend thread and forwards only
        // the events that change which files exist (create/remove/rename),
        // ignoring metadata/access noise.
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                if let Ok(event) = res {
                    if matches!(
                        event.kind,
                        EventKind::Create(_)
                            | EventKind::Remove(_)
                            | EventKind::Modify(ModifyKind::Name(_))
                    ) {
                        let _ = tx.send(Message::FileChanged(format!("{event:?}")));
                    }
                }
            })?;

        for path in &paths {
            if path.exists() {
                watcher.watch(path, RecursiveMode::NonRecursive)?;
            }
        }

        Ok(Self {
            watcher,
            watched_paths: paths,
        })
    }

    pub fn set_paths(&mut self, paths: Vec<PathBuf>) -> Result<()> {
        // Drop the old watches, then start watching the new set. Unlike the
        // previous inotify implementation this takes effect immediately.
        for path in &self.watched_paths {
            let _ = self.watcher.unwatch(path);
        }
        for path in &paths {
            if path.exists() {
                self.watcher.watch(path, RecursiveMode::NonRecursive)?;
            }
        }
        self.watched_paths = paths;
        Ok(())
    }
}
