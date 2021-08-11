use eframe::epi::RepaintSignal;

type Repainter = std::sync::Arc<dyn RepaintSignal>;
cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        pub struct PeriodicUpdater {

        }

        impl PeriodicUpdater {
            fn new(repaint_signal: Repainter) -> Self {
                PeriodicUpdater{}
            }
        }


        impl Drop for PeriodicUpdater {
            fn drop(&mut self) {
            }
        }        
    } else {
        use crossbeam::channel::{self, Sender};
        use std::{thread::JoinHandle, time::Duration};

        pub struct PeriodicUpdater {
            quitter: Sender<()>,
            join_handle: Option<JoinHandle<()>>,
        }

        impl PeriodicUpdater {
            pub fn new(repaint_signal: Repainter) -> Self {
                let (tx, rx) = channel::bounded(1);
                let join_handle = Some(std::thread::spawn(move || {
                        while rx.try_recv().is_err() {
                            std::thread::sleep(Duration::from_millis(100));
                            repaint_signal.request_repaint();
                        }
                    }));
                PeriodicUpdater{quitter: tx, join_handle}
            }
        }

        impl Drop for PeriodicUpdater {
            fn drop(&mut self) {
                self.quitter.send(()).unwrap();
                self.join_handle.take().unwrap().join().unwrap();                
            }
        }
    }
}