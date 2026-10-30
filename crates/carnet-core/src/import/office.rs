//! Conversion par LibreOffice installé sur le poste : le fichier devient un PDF, puis l'import PDF de
//! LibreOffice Draw en fait un ODG, lu par `read_odg`. Profil à part, sans interface, délai limite.

use super::carnet::read_carnet;
use super::odg::read_odg;
use super::{ImportError, Imported};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(windows)]
pub const SOFFICE_EXE: &str = "soffice.exe";
#[cfg(not(windows))]
pub const SOFFICE_EXE: &str = "soffice";

/// Délai limite de chaque fichier.
pub const TIMEOUT: Duration = Duration::from_secs(120);

/// Formats confiés à LibreOffice.
const OFFICE_EXTENSIONS: [&str; 6] = ["docx", "doc", "odt", "odg", "pdf", "pub"];

/// Où chercher LibreOffice ; `from_env` lit le poste, les tests fournissent leurs dossiers.
pub struct SofficeLookup {
    pub path_var: Option<OsString>,
    pub program_files: Vec<PathBuf>,
    /// Dossier `program` indiqué par le registre de Windows.
    pub registry: Option<PathBuf>,
}

impl SofficeLookup {
    pub fn from_env() -> Self {
        let program_files = ["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"].iter().filter_map(std::env::var_os).map(PathBuf::from).collect();
        SofficeLookup { path_var: std::env::var_os("PATH"), program_files, registry: registry_program_dir() }
    }
}

/// `reg query HKLM\SOFTWARE\LibreOffice\UNO\InstallPath /ve` : ligne « (par défaut) REG_SZ C:\…\program ».
#[cfg(windows)]
fn registry_program_dir() -> Option<PathBuf> {
    let mut cmd = Command::new("reg");
    cmd.args(["query", r"HKLM\SOFTWARE\LibreOffice\UNO\InstallPath", "/ve"]);
    no_window(&mut cmd);
    let out = cmd.stderr(Stdio::null()).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().find(|l| l.contains("REG_SZ"))?;
    Some(PathBuf::from(line.split("REG_SZ").nth(1)?.trim()))
}

#[cfg(not(windows))]
fn registry_program_dir() -> Option<PathBuf> {
    None
}

pub fn find_soffice(l: &SofficeLookup) -> Option<PathBuf> {
    let installed = l.program_files.iter().map(|pf| pf.join("LibreOffice").join("program").join(SOFFICE_EXE));
    let registered = l.registry.iter().map(|d| d.join(SOFFICE_EXE));
    let on_path = l.path_var.iter().flat_map(std::env::split_paths).map(|d| d.join(SOFFICE_EXE));
    installed.chain(registered).chain(on_path).find(|p| p.is_file())
}

/// URL `file:///` d'un dossier, encodée (espaces et accents d'un nom d'utilisateur Windows).
pub fn profile_url(dir: &Path) -> String {
    let path = dir.to_string_lossy().replace('\\', "/");
    let mut out = String::from("file://");
    if !path.starts_with('/') {
        out.push('/');
    }
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || b"/:-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    ToPdf,
    PdfToOdg,
}

pub fn soffice_args(profile: &str, step: Step, input: &Path, outdir: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec![format!("-env:UserInstallation={profile}").into(), "--headless".into(), "--norestore".into(), "--nolockcheck".into()];
    match step {
        Step::ToPdf => args.extend(["--convert-to".into(), "pdf".into()]),
        Step::PdfToOdg => args.extend(["--infilter=draw_pdf_import".into(), "--convert-to".into(), "odg".into()]),
    }
    args.extend(["--outdir".into(), outdir.as_os_str().to_owned(), input.as_os_str().to_owned()]);
    args
}

#[cfg(windows)]
fn no_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

/// LibreOffice lance un sous-processus (`soffice.bin`) : tout l'arbre est arrêté.
fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("taskkill");
        cmd.args(["/T", "/F", "/PID", &child.id().to_string()]).stdout(Stdio::null()).stderr(Stdio::null());
        no_window(&mut cmd);
        let _ = cmd.status();
    }
    #[cfg(unix)]
    {
        // Le processus est chef de son propre groupe (voir `run`).
        let _ = Command::new("kill").args(["-KILL", "--", &format!("-{}", child.id())]).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn run(soffice: &Path, args: Vec<OsString>, deadline: Instant, cancel: &AtomicBool) -> Result<(), ImportError> {
    let mut cmd = Command::new(soffice);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    no_window(&mut cmd);
    let mut child = cmd.spawn().map_err(|_| ImportError::Conversion)?;
    loop {
        if cancel.load(Ordering::SeqCst) {
            kill_tree(&mut child);
            return Err(ImportError::Cancelled);
        }
        if Instant::now() >= deadline {
            kill_tree(&mut child);
            return Err(ImportError::Timeout);
        }
        match child.try_wait() {
            Ok(Some(_)) => return Ok(()),
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => return Err(ImportError::Conversion),
        }
    }
}

fn extension(path: &Path) -> String {
    path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase()
}

/// Réglages du profil LibreOffice d'import : aucun contenu lié (image distante, lien de suivi) n'est
/// téléchargé, et pas de recherche de mise à jour. Écrits avant le premier lancement.
const OFFLINE_SETTINGS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Common/Security/Scripting"><prop oor:name="BlockUntrustedRefererLinks" oor:op="fuse"><value>true</value></prop></item>
<item oor:path="/org.openoffice.Office.Jobs/Jobs/org.openoffice.Office.Jobs:Job['UpdateCheck']/Arguments"><prop oor:name="AutoCheckEnabled" oor:op="fuse"><value>false</value></prop></item>
</oor:items>
"#;

fn write_offline_profile(profile_dir: &Path) -> Result<(), ImportError> {
    let user = profile_dir.join("user");
    std::fs::create_dir_all(&user).map_err(|_| ImportError::Conversion)?;
    std::fs::write(user.join("registrymodifications.xcu"), OFFLINE_SETTINGS).map_err(|_| ImportError::Conversion)
}

/// Copie de l'entrée sous un nom neutre, PDF si besoin, puis ODG par l'import PDF de Draw.
pub fn convert_to_odg(soffice: &Path, input: &Path, work: &Path, timeout: Duration, cancel: &AtomicBool) -> Result<PathBuf, ImportError> {
    let deadline = Instant::now() + timeout;
    let ext = extension(input);
    let source = work.join(format!("source.{ext}"));
    std::fs::copy(input, &source).map_err(|_| ImportError::Conversion)?;
    write_offline_profile(&work.join("profil"))?;
    let profile = profile_url(&work.join("profil"));
    let pdf = if ext == "pdf" {
        source
    } else {
        run(soffice, soffice_args(&profile, Step::ToPdf, &source, work), deadline, cancel)?;
        let pdf = work.join("source.pdf");
        if !pdf.is_file() {
            return Err(ImportError::Conversion);
        }
        pdf
    };
    let outdir = work.join("odg");
    run(soffice, soffice_args(&profile, Step::PdfToOdg, &pdf, &outdir), deadline, cancel)?;
    let odg = outdir.join("source.odg");
    if odg.is_file() { Ok(odg) } else { Err(ImportError::Conversion) }
}

/// Dossier de travail d'un import (copie du fichier, profil LibreOffice), supprimé à la fin, réussite ou
/// non. Juste après l'arrêt de LibreOffice, Windows peut encore tenir des fichiers ouverts : la
/// suppression réessaie pendant 2 s.
struct WorkDir(PathBuf);

impl WorkDir {
    fn new() -> Result<WorkDir, ImportError> {
        let dir = tempfile::Builder::new().prefix("carnet-import").tempdir().map_err(|_| ImportError::Conversion)?;
        Ok(WorkDir(dir.keep()))
    }
}

impl Drop for WorkDir {
    fn drop(&mut self) {
        for _ in 0..20 {
            match std::fs::remove_dir_all(&self.0) {
                Ok(()) => return,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
                Err(_) => std::thread::sleep(Duration::from_millis(100)),
            }
        }
    }
}

/// Un fichier à importer : `.carnet` directement, les autres par LibreOffice (dossier temporaire supprimé à la fin).
pub fn import_file(path: &Path, soffice: Option<&Path>, known_fonts: &[String], cancel: &AtomicBool) -> Result<Imported, ImportError> {
    let ext = extension(path);
    if ext == crate::native::DOC_EXT {
        let file = std::fs::File::open(path).map_err(|_| ImportError::Native(crate::native::NativeError::Io))?;
        return read_carnet(file);
    }
    if !OFFICE_EXTENSIONS.contains(&ext.as_str()) {
        return Err(ImportError::Unsupported);
    }
    let soffice = soffice.ok_or(ImportError::NoLibreOffice)?;
    let work = WorkDir::new()?;
    let odg = convert_to_odg(soffice, path, &work.0, TIMEOUT, cancel)?;
    let file = std::fs::File::open(odg).map_err(|_| ImportError::Unreadable)?;
    read_odg(file, known_fonts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;
    use std::time::Instant;

    #[test]
    fn finds_soffice_in_program_files_then_registry_then_path() {
        let root = tempfile::tempdir().unwrap();
        let mk = |dir: &str| {
            let d = root.path().join(dir);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join(SOFFICE_EXE), b"").unwrap();
            d
        };
        let pf_program = mk("pf/LibreOffice/program");
        let reg = mk("reg");
        let on_path = mk("bin");
        let empty_pf = root.path().join("vide");
        let path_var = std::env::join_paths([root.path().join("ailleurs"), on_path.clone()]).unwrap();
        let all = SofficeLookup { path_var: Some(path_var.clone()), program_files: vec![empty_pf.clone(), root.path().join("pf")], registry: Some(reg.clone()) };
        assert_eq!(find_soffice(&all), Some(pf_program.join(SOFFICE_EXE)));
        let no_pf = SofficeLookup { program_files: vec![empty_pf.clone()], ..all };
        assert_eq!(find_soffice(&no_pf), Some(reg.join(SOFFICE_EXE)));
        let path_only = SofficeLookup { registry: None, ..no_pf };
        assert_eq!(find_soffice(&path_only), Some(on_path.join(SOFFICE_EXE)));
        assert_eq!(find_soffice(&SofficeLookup { path_var: None, program_files: vec![], registry: None }), None);
    }

    #[test]
    fn profile_url_is_encoded() {
        assert_eq!(profile_url(Path::new(r"C:\Users\Jean Dupont\AppData\Local\Temp\x")), "file:///C:/Users/Jean%20Dupont/AppData/Local/Temp/x");
        assert_eq!(profile_url(Path::new("/tmp/carnet été/profil")), "file:///tmp/carnet%20%C3%A9t%C3%A9/profil");
    }

    #[test]
    fn unsupported_extension_is_refused() {
        let cancel = AtomicBool::new(false);
        let err = import_file(Path::new("/nulle/part/classe.xlsx"), Some(Path::new("/bin/true")), &[], &cancel).err().unwrap();
        assert_eq!(err, ImportError::Unsupported);
    }

    #[test]
    fn office_file_without_libreoffice_says_how_to_fix() {
        let dir = tempfile::tempdir().unwrap();
        let docx = dir.path().join("fiche.docx");
        std::fs::write(&docx, b"PK").unwrap();
        let err = import_file(&docx, None, &[], &AtomicBool::new(false)).err().unwrap();
        assert_eq!(err, ImportError::NoLibreOffice);
        assert_eq!(err.message_fr(), "Pour importer ce fichier, installez LibreOffice (gratuit), puis recommencez. Les carnets s'importent sans LibreOffice.");
    }

    /// Faux soffice : écrit ses arguments, un par ligne, dans `log`, puis exécute `body`.
    /// Le script est écrit par un `sh` à part : si ce processus de tests le gardait ouvert en écriture, un
    /// programme lancé au même moment par un autre test en hériterait, et son exécution échouerait (ETXTBSY).
    #[cfg(unix)]
    fn fake_soffice(dir: &Path, log: &Path, body: &str) -> PathBuf {
        use std::io::Write;
        let script = dir.join("soffice");
        let mut sh = Command::new("sh")
            .args(["-c", "cat > \"$1\" && chmod 755 \"$1\"", "sh"])
            .arg(&script)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let text = format!("#!/bin/sh\nfor a in \"$@\"; do echo \"$a\" >> '{}'; done\n{body}\n", log.display());
        sh.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
        assert!(sh.wait().unwrap().success());
        script
    }

    #[cfg(unix)]
    #[test]
    fn soffice_gets_fixed_arguments() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("args.txt");
        let soffice = fake_soffice(dir.path(), &log, "exit 0");
        let input = dir.path().join("Fiche été.docx");
        std::fs::write(&input, b"PK").unwrap();
        let _ = import_file(&input, Some(&soffice), &[], &AtomicBool::new(false));
        let args: Vec<String> = std::fs::read_to_string(&log).unwrap().lines().map(String::from).collect();
        assert!(args[0].starts_with("-env:UserInstallation=file:///"), "{args:?}");
        for flag in ["--headless", "--norestore", "--nolockcheck", "--convert-to", "pdf", "--outdir"] {
            assert!(args.iter().any(|a| a == flag), "{flag} absent : {args:?}");
        }
        assert!(args.last().unwrap().ends_with("source.docx"), "{args:?}");
        assert!(!args.iter().any(|a| a.contains("Fiche")), "{args:?}");
    }

    #[cfg(unix)]
    #[test]
    fn conversion_without_output_is_a_clear_error() {
        let dir = tempfile::tempdir().unwrap();
        let log = dir.path().join("args.txt");
        let soffice = fake_soffice(dir.path(), &log, "exit 0");
        let input = dir.path().join("fiche.pdf");
        std::fs::write(&input, b"%PDF-1.4").unwrap();
        let err = import_file(&input, Some(&soffice), &[], &AtomicBool::new(false)).err().unwrap();
        assert_eq!(err, ImportError::Conversion);
        let args = std::fs::read_to_string(&log).unwrap();
        let outdir = args.lines().skip_while(|a| *a != "--outdir").nth(1).unwrap().to_string();
        assert!(args.contains("--infilter=draw_pdf_import"), "{args}");
        assert!(!Path::new(&outdir).exists(), "dossier de travail resté : {outdir}");
    }

    #[cfg(unix)]
    #[test]
    fn cancel_kills_the_conversion() {
        let dir = tempfile::tempdir().unwrap();
        let soffice = fake_soffice(dir.path(), &dir.path().join("args.txt"), "exec sleep 30");
        let input = dir.path().join("fiche.pdf");
        std::fs::write(&input, b"%PDF-1.4").unwrap();
        let cancel = AtomicBool::new(false);
        let start = Instant::now();
        let err = std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(200));
                cancel.store(true, Ordering::SeqCst);
            });
            import_file(&input, Some(&soffice), &[], &cancel).err().unwrap()
        });
        assert_eq!(err, ImportError::Cancelled);
        assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
    }

    /// Fichiers encore tenus un instant après l'arrêt de LibreOffice (Windows) : la suppression réessaie.
    #[cfg(unix)]
    #[test]
    fn work_dir_is_removed_even_when_briefly_locked() {
        use std::os::unix::fs::PermissionsExt;
        let parent = tempfile::tempdir().unwrap();
        let work = parent.path().join("travail");
        let locked = work.join("profil");
        std::fs::create_dir_all(&locked).unwrap();
        std::fs::write(locked.join("source.docx"), b"PK").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500)).unwrap();
        std::thread::scope(|s| {
            s.spawn(|| {
                std::thread::sleep(Duration::from_millis(150));
                std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700)).unwrap();
            });
            drop(WorkDir(work.clone()));
        });
        assert!(!work.exists());
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_the_conversion() {
        let dir = tempfile::tempdir().unwrap();
        let soffice = fake_soffice(dir.path(), &dir.path().join("args.txt"), "exec sleep 30");
        let input = dir.path().join("fiche.pdf");
        std::fs::write(&input, b"%PDF-1.4").unwrap();
        let work = tempfile::tempdir().unwrap();
        let start = Instant::now();
        let err = convert_to_odg(&soffice, &input, work.path(), Duration::from_millis(300), &AtomicBool::new(false)).err().unwrap();
        assert_eq!(err, ImportError::Timeout);
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
