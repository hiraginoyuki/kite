use anyhow::Context;
use async_channel::{Receiver, Sender};
use derive_more::Deref;
use notify::RecursiveMode::NonRecursive;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    thread,
};

pub struct WatchManager<W: notify::Watcher> {
    watcher: W,
    watcher_rx: Receiver<notify::Event>,
    eyes: HashMap<PathBuf, (Sender<notify::Event>, Eye)>,
    manager_tx: Sender<EyeMsg>,
    manager_rx: Receiver<EyeMsg>,
}

impl<W: notify::Watcher> WatchManager<W> {
    pub fn new() -> notify::Result<Self> {
        let (tx, watcher_rx) = async_channel::unbounded();
        let watcher = W::new(
            move |e| {
                if let Ok(e) = e {
                    tx.send_blocking(e)
                        .context("rx closed unexpectedly")
                        .unwrap();
                }
            },
            Default::default(),
        )?;

        let (manager_tx, manager_rx) = async_channel::unbounded();

        let thread = {
            let watcher_rx = watcher_rx.clone();
            let manager_rx = manager_rx.clone();
            move || {
                watcher_rx;
            }
        };

        Ok(Self {
            watcher,
            watcher_rx,
            eyes: HashMap::new(),
            manager_tx,
            manager_rx,
        })
    }

    pub fn watch(&mut self, path: impl AsRef<Path>) -> notify::Result<Eye> {
        self.watcher
            .watch(path.as_ref().parent().unwrap(), NonRecursive)?;

        Ok(self
            .eyes
            .entry(path.as_ref().to_owned())
            .or_insert_with(|| {
                let (eye_tx, eye_rx) = async_channel::unbounded();
                (
                    eye_tx,
                    Eye {
                        path: path.as_ref().to_owned(),
                        eye_rx,
                        manager_tx: self.manager_tx.clone(),
                    },
                )
            })
            .1
            .clone())
    }
}

enum EyeMsg {
    Drop(PathBuf),
}

#[derive(Deref, Clone)]
pub struct Eye {
    path: PathBuf,
    #[deref]
    eye_rx: Receiver<notify::Event>,
    manager_tx: Sender<EyeMsg>,
}
impl Drop for Eye {
    fn drop(&mut self) {
        let _ = self
            .manager_tx
            .send_blocking(EyeMsg::Drop(self.path.clone()));
    }
}
