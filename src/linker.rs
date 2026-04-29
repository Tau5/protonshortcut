use std::os::unix::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use steamlocate::{App, Library, Shortcut};
use crate::linker::SuccessType::{Created, Deleted, Skipped};

pub struct Linker {
    sender: mpsc::Sender<String>,
    delete: bool,
    successes: usize
}

enum SuccessType {
    Created,
    Deleted,
    Skipped
}

impl Linker {
    fn log(&self, string: String) {
       self.sender.send(string);
    }

    fn log_success(&mut self, success_type: SuccessType, display_name: String) {
        self.successes += 1;
        match success_type {
            SuccessType::Created => self.log(format!("Created link for {}", display_name)),
            SuccessType::Deleted => self.log(format!("Removed link for {}", display_name)),
            SuccessType::Skipped => {}
        }
    }

    fn process_shortcut(&mut self, shortcut: &Shortcut, libraries: &Vec<Library>) -> Result<SuccessType, String> {
        let executable_string = shortcut.executable.trim_matches(|c| c == '\"' || c == '\'').to_string();
        let excutable = PathBuf::from(executable_string);

        let install_path = excutable.parent()
            .ok_or(format!("Couldn't find install path for {}", shortcut.app_name))?;

        if !install_path.is_dir() || !install_path.exists() {
            return Err(format!("Install path for {} not found ({})", shortcut.app_name, install_path.to_str().unwrap_or("")))
        };

        if let Some(prefix)  = libraries.iter().find_map(|l| {
            let mut compatdata = l.path().join("steamapps").join("compatdata");
            compatdata.push(shortcut.app_id.to_string());
            if compatdata.exists() { Some(compatdata) } else { None }
        }) {
            let linkname = install_path.join("compatdata");
            self.create_or_delete_link(prefix, shortcut.app_name.clone(), linkname)
        } else {
            Ok(Skipped)
        }
    }

    fn process_app(&mut self, library: &Library, app: &App) -> Result<SuccessType, String> {
        let compatdata = library.path().join("steamapps").join("compatdata");

        let installdir = library.resolve_app_dir(&app);
        let prefix = compatdata.join(app.app_id.to_string());

        let display_name = app.name.clone().unwrap_or(app.app_id.to_string());
        let linkname = installdir.join("compatdata");

        self.create_or_delete_link(prefix, display_name.clone(), linkname)
    }

    fn create_or_delete_link(&mut self, prefix: PathBuf, display_name: String, linkname: PathBuf) -> Result<SuccessType, String> {
        if !self.delete {
            if !prefix.exists() || linkname.exists() {
                return Ok(Skipped)
            }

            if let Err(e) = fs::symlink(prefix, linkname) {
                Err(
                    format!("Error creating link for {} ({})", display_name, e).into()
                )
            } else {
                Ok(Created)
            }
        } else {
            if let Err(e) = std::fs::remove_file(linkname) {
                Err(
                    format!("Error removing link for {} ({})", display_name, e).into()
                )
            } else {
                Ok(Deleted)
            }
        }
    }

    pub fn scan_and_process_apps(&mut self) {
        let steamdirs = steamlocate::locate_all().expect("Could not find steamapps");

        for steam_dir in steamdirs {
            let libraries: Vec<Library> = steam_dir.libraries().iter_mut().flatten().filter_map(Result::ok).collect();

            libraries
                .iter().for_each(|library| {
                    self.log(format!("Found library {}", library.path().to_str().unwrap()));
                    library
                        .apps()
                        .filter_map(Result::ok)
                        .for_each(|a|
                            match self.process_app(&library, &a) {
                                Ok(s) => self.log_success(s, a.name.unwrap()),
                                Err(e) => self.log(format!("Error: {}", e))
                            }
                        );
                });

            steam_dir.shortcuts()
                .iter_mut()
                .flatten()
                .filter_map(Result::ok)
                .for_each(|shortcut| {
                    match self.process_shortcut(&shortcut, &libraries) {
                        Ok(s) => self.log_success(s, shortcut.app_name),
                        Err(e) => self.log(format!("Error: {}", e))
                    }
                });
        }
        self.log(format!("Processed {} apps succesfully", self.successes));
    }

    pub fn new(sender: mpsc::Sender<String>, delete: bool) -> Linker {
        Self {
            sender,
            delete,
            successes: 0
        }
    }
}