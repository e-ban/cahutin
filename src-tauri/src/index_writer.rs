//! Enregistrement de l'index de la photothèque hors du clic : un clic (étiqueter, retirer) ou un paquet du
//! parcours donne l'index mis en forme et repart sans attendre le disque ; un seul fil l'écrit. Pendant une
//! écriture, les demandes suivantes ne gardent que le dernier état de chaque photothèque, écrit juste après :
//! jamais deux écritures à la fois, jamais un état plus ancien écrit après un plus récent. Chaque écriture passe
//! par un fichier temporaire renommé (`write_index`) : un arrêt brutal ne perd que le dernier état en attente,
//! jamais l'index entier.
//!
//! Les autres enregistrements (dossiers suivis, migration) attendent le leur (`save_now`), et ce qui attend est
//! écrit avant toute relecture du disque (`flush`) : à l'ouverture d'une classe, avant de reporter un parcours
//! dans l'index d'une classe qui n'est plus ouverte, à la fermeture de l'application.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;

type Write = dyn Fn(&Path, &[u8]) -> Result<(), String> + Send + Sync;

#[derive(Default)]
struct Slot {
    /// Le dernier état demandé de chaque photothèque, pas encore écrit : son dossier et son index mis en forme.
    pending: Vec<(PathBuf, Vec<u8>)>,
    /// Un état nouveau attend le fil.
    wake: bool,
    stop: bool,
    /// L'écriture en arrière-plan a échoué : dit au clic suivant.
    failed: Option<String>,
}

struct Shared {
    slot: Mutex<Slot>,
    cond: Condvar,
    /// Tenu pendant chaque écriture : une seule à la fois, et ce qui attend n'est pris que sous ce verrou (l'ordre
    /// des écritures est celui des états).
    file: Mutex<()>,
    write: Box<Write>,
}

impl Shared {
    fn lock(&self) -> std::sync::MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Remet un état dont l'écriture a échoué, sauf si un plus récent attend déjà pour ce dossier.
    fn put_back(&self, dir: PathBuf, json: Vec<u8>) {
        let mut slot = self.lock();
        if !slot.pending.iter().any(|(d, _)| *d == dir) {
            slot.pending.push((dir, json));
        }
    }

    /// Écrit ce qui attend ; ce qui échoue reste à écrire. Rend la première erreur. À appeler sous `file`.
    fn write_pending(&self) -> Result<(), String> {
        let pending = std::mem::take(&mut self.lock().pending);
        let mut result = Ok(());
        for (dir, json) in pending {
            if let Err(e) = (self.write)(&dir, &json) {
                self.put_back(dir, json);
                result = result.and(Err(e));
            }
        }
        result
    }
}

pub struct IndexWriter {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl IndexWriter {
    pub fn new() -> IndexWriter {
        IndexWriter::with_write(carnet_core::phototheque::write_index)
    }

    /// Avec la façon d'écrire donnée (un test la ralentit ou la retient).
    pub fn with_write(write: impl Fn(&Path, &[u8]) -> Result<(), String> + Send + Sync + 'static) -> IndexWriter {
        let shared = Arc::new(Shared { slot: Mutex::default(), cond: Condvar::new(), file: Mutex::new(()), write: Box::new(write) });
        let worker = shared.clone();
        let thread = std::thread::spawn(move || {
            loop {
                {
                    let mut slot = worker.lock();
                    while !slot.wake && !slot.stop {
                        slot = worker.cond.wait(slot).unwrap_or_else(|e| e.into_inner());
                    }
                    if !slot.wake {
                        return;
                    }
                    slot.wake = false;
                }
                let _file = worker.file.lock().unwrap_or_else(|e| e.into_inner());
                if let Err(e) = worker.write_pending() {
                    log::error!("phototheque_save_failed");
                    worker.lock().failed = Some(e);
                }
            }
        });
        IndexWriter { shared, thread: Some(thread) }
    }

    /// L'état à enregistrer après un clic, sans attendre le disque. Rend l'erreur d'un enregistrement précédent
    /// qui a échoué (l'état est gardé et réessayé).
    pub fn request(&self, dir: &Path, json: Vec<u8>) -> Result<(), String> {
        let mut slot = self.shared.lock();
        slot.pending.retain(|(d, _)| d != dir);
        slot.pending.push((dir.to_path_buf(), json));
        slot.wake = true;
        let failed = slot.failed.take();
        drop(slot);
        self.shared.cond.notify_one();
        failed.map_or(Ok(()), Err)
    }

    /// Écrit tout de suite ce qui attend, et attend la fin d'une écriture en cours.
    pub fn flush(&self) -> Result<(), String> {
        let _file = self.shared.file.lock().unwrap_or_else(|e| e.into_inner());
        self.shared.write_pending()
    }

    /// Enregistre `json` et l'attend : un état plus récent que tout ce qui attend pour ce dossier, qui le remplace.
    pub fn save_now(&self, dir: &Path, json: &[u8]) -> Result<(), String> {
        let _file = self.shared.file.lock().unwrap_or_else(|e| e.into_inner());
        let superseded = {
            let mut slot = self.shared.lock();
            let at = slot.pending.iter().position(|(d, _)| d == dir);
            at.map(|i| slot.pending.remove(i))
        };
        // L'état remplacé revient si celui-ci n'est pas écrit (l'appelant peut ne pas garder le sien).
        (self.shared.write)(dir, json).inspect_err(|_| {
            if let Some((d, j)) = superseded {
                self.shared.put_back(d, j);
            }
        })
    }
}

impl Drop for IndexWriter {
    fn drop(&mut self) {
        self.shared.lock().stop = true;
        self.shared.cond.notify_one();
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        if self.flush().is_err() {
            log::error!("phototheque_save_failed at=drop");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    /// Une écriture retenue : chaque écriture annonce son contenu, puis attend la permission de finir.
    fn held() -> (IndexWriter, mpsc::Receiver<Vec<u8>>, mpsc::Sender<()>) {
        let (started, writes) = mpsc::channel();
        let (go, gate) = mpsc::channel::<()>();
        let gate = Mutex::new(gate);
        let w = IndexWriter::with_write(move |_, json| {
            started.send(json.to_vec()).unwrap();
            gate.lock().unwrap().recv().map_err(|_| "fermé".to_string())
        });
        (w, writes, go)
    }

    #[test]
    fn clicks_during_a_slow_write_end_in_one_more_write_of_the_latest_state() {
        let (w, writes, go) = held();
        let dir = Path::new("/classe");
        w.request(dir, b"1".to_vec()).unwrap();
        assert_eq!(writes.recv_timeout(Duration::from_secs(5)).unwrap(), b"1");
        // L'écriture de « 1 » est retenue : les clics suivants n'attendent pas.
        for n in 2..=5 {
            w.request(dir, n.to_string().into_bytes()).unwrap();
        }
        go.send(()).unwrap();
        assert_eq!(writes.recv_timeout(Duration::from_secs(5)).unwrap(), b"5");
        go.send(()).unwrap();
        w.flush().unwrap();
        assert!(writes.try_recv().is_err(), "deux écritures en tout");
    }

    #[test]
    fn a_save_now_replaces_an_older_pending_state_and_is_never_overwritten_by_it() {
        let written = Arc::new(Mutex::new(vec![]));
        let log = written.clone();
        let w = IndexWriter::with_write(move |d, json| {
            log.lock().unwrap().push((d.to_path_buf(), json.to_vec()));
            Ok(())
        });
        let dir = Path::new("/classe");
        w.request(dir, b"ancien".to_vec()).unwrap();
        w.save_now(dir, b"nouveau").unwrap();
        w.flush().unwrap();
        drop(w);
        let written = written.lock().unwrap();
        assert_eq!(written.last().unwrap().1, b"nouveau");
        assert!(written.iter().all(|(_, j)| j == b"ancien" || j == b"nouveau"));
        let at = |j: &[u8]| written.iter().position(|(_, x)| x == j);
        assert!(at(b"ancien").is_none_or(|a| a < at(b"nouveau").unwrap()));
    }

    #[test]
    fn a_failed_background_write_is_told_to_the_next_click_and_kept_for_the_next_flush() {
        let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let written = Arc::new(Mutex::new(vec![]));
        let (f, log) = (fail.clone(), written.clone());
        let (done, finished) = mpsc::channel();
        let w = IndexWriter::with_write(move |_, json| {
            let _ = done.send(());
            if f.load(std::sync::atomic::Ordering::SeqCst) {
                return Err("disque plein".to_string());
            }
            log.lock().unwrap().push(json.to_vec());
            Ok(())
        });
        let dir = Path::new("/classe");
        w.request(dir, b"1".to_vec()).unwrap();
        finished.recv_timeout(Duration::from_secs(5)).unwrap();
        // Attend la fin de l'écriture ratée ; l'état « 1 » est gardé, et échoue encore ici.
        assert!(w.flush().is_err());
        fail.store(false, std::sync::atomic::Ordering::SeqCst);
        // Le clic suivant apprend l'échec ; son état, plus récent, remplace « 1 ».
        assert_eq!(w.request(dir, b"2".to_vec()).unwrap_err(), "disque plein");
        w.flush().unwrap();
        assert_eq!(*written.lock().unwrap(), [b"2".to_vec()]);
    }

    #[test]
    fn what_is_pending_is_written_when_the_writer_goes() {
        let written = Arc::new(Mutex::new(vec![]));
        let log = written.clone();
        let (go, gate) = mpsc::channel::<()>();
        let gate = Mutex::new(gate);
        let w = IndexWriter::with_write(move |_, json| {
            let _ = gate.lock().unwrap().recv_timeout(Duration::from_millis(200));
            log.lock().unwrap().push(json.to_vec());
            Ok(())
        });
        w.request(Path::new("/classe"), b"1".to_vec()).unwrap();
        w.request(Path::new("/classe"), b"2".to_vec()).unwrap();
        drop(go);
        drop(w);
        assert_eq!(written.lock().unwrap().last().unwrap(), b"2");
    }
}
