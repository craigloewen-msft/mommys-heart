use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::time::Duration;

use leptos::prelude::*;
use leptos::task::spawn_local;

const DELAY: Duration = Duration::from_millis(1500);
const RETRY_DELAY: Duration = Duration::from_secs(5);

pub type SaveFuture = Pin<Box<dyn Future<Output = Result<(), String>>>>;
type SaveFn = Rc<dyn Fn() -> Option<SaveFuture>>;

#[derive(Clone, Copy)]
pub struct Autosaver {
    save: StoredValue<SaveFn, LocalStorage>,
    paused: RwSignal<bool>,
    generation: RwSignal<u32>,
    queued: RwSignal<bool>,
    pub saving: RwSignal<bool>,
    pub dirty: RwSignal<bool>,
    pub status: RwSignal<String>,
    pub failed: RwSignal<Option<String>>,
}

impl Autosaver {
    pub fn new(paused: RwSignal<bool>, save: impl Fn() -> Option<SaveFuture> + 'static) -> Self {
        let saver = Self {
            save: StoredValue::new_local(Rc::new(save)),
            paused,
            generation: RwSignal::new(0),
            queued: RwSignal::new(false),
            saving: RwSignal::new(false),
            dirty: RwSignal::new(false),
            status: RwSignal::new(String::new()),
            failed: RwSignal::new(None),
        };
        Effect::new(move |was_paused: Option<bool>| {
            let now_paused = paused.get();
            if was_paused == Some(true) && !now_paused && saver.dirty.get_untracked() {
                saver.save_now();
            }
            now_paused
        });
        let unload = window_event_listener(leptos::ev::beforeunload, move |event| {
            if saver.dirty.try_get_untracked() == Some(true) {
                event.prevent_default();
                event.set_return_value("");
            }
        });
        on_cleanup(move || {
            unload.remove();
            saver.flush();
        });
        saver
    }

    pub fn changed(self) {
        self.dirty.set(true);
        self.generation.update(|g| *g += 1);
        let generation = self.generation.get_untracked();
        set_timeout(
            move || {
                if self.generation.try_get_untracked() == Some(generation) {
                    self.run(false);
                }
            },
            DELAY,
        );
    }

    pub fn save_now(self) {
        self.run(false);
    }

    pub fn snapshot(self) -> u32 {
        self.generation.get_untracked()
    }

    pub fn saved_through(self, snapshot: u32) {
        if self.generation.try_get_untracked() == Some(snapshot) {
            let _ = self.dirty.try_set(false);
            let _ = self.failed.try_set(None);
            let _ = self.status.try_set(format!("Saved {}", clock()));
        } else {
            self.save_now();
        }
    }

    fn run(self, is_retry: bool) {
        if self.paused.try_get_untracked() != Some(false) {
            return;
        }
        if self.saving.get_untracked() {
            self.queued.set(true);
            return;
        }
        let Some(future) = self.save.try_with_value(|save| save()).flatten() else {
            return;
        };
        let started = self.generation.get_untracked();
        self.saving.set(true);
        self.status.set("Saving\u{2026}".to_string());
        spawn_local(async move {
            let result = future.await;
            let _ = self.saving.try_set(false);
            let Some(latest) = self.generation.try_get_untracked() else {
                return;
            };
            let queued = self.queued.try_get_untracked() == Some(true);
            let _ = self.queued.try_set(false);
            match result {
                Ok(()) => {
                    let _ = self.failed.try_set(None);
                    if latest == started {
                        let _ = self.dirty.try_set(false);
                        let _ = self.status.try_set(format!("Saved {}", clock()));
                    } else {
                        self.run(false);
                    }
                }
                Err(e) => {
                    let _ = self.status.try_set(format!("Not saved: {e}"));
                    if is_retry {
                        let _ = self.failed.try_set(Some(e));
                        if queued {
                            self.run(false);
                        }
                    } else if queued || latest != started {
                        self.run(false);
                    } else {
                        set_timeout(move || self.run(true), RETRY_DELAY);
                    }
                }
            }
        });
    }

    fn flush(self) {
        if self.dirty.try_get_untracked() != Some(true)
            || self.paused.try_get_untracked() != Some(false)
            || self.saving.try_get_untracked() != Some(false)
        {
            return;
        }
        if let Some(future) = self.save.try_with_value(|save| save()).flatten() {
            spawn_local(async move {
                let _ = future.await;
            });
        }
    }
}

fn clock() -> String {
    crate::state::now_stamp().get(11..16).unwrap_or_default().to_string()
}
