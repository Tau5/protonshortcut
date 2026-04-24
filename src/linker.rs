use std::os::unix::fs;
use std::sync::mpsc;
use steamlocate::{App, Library};

pub struct Linker {
    sender: mpsc::Sender<String>,
    delete: bool,
    successes: usize
}

impl Linker {
    fn log(&self, string: String) {
       self.sender.send(string);
    }

    fn process_app(&mut self, library: &Library, app: &App) {
        let compatdata = library.path().join("steamapps").join("compatdata");

        let installdir = library.resolve_app_dir(&app);
        let prefix = compatdata.join(app.app_id.to_string());

        let display_name = app.name.clone().unwrap_or(app.app_id.to_string());
        let linkname = installdir.join("compatdata");

        if prefix.is_dir() {
            if !self.delete {
                if linkname.exists() {
                    self.successes += 1;
                    return;
                }

                if let Err(e) = fs::symlink(prefix, linkname) {
                    self.log(
                        format!("Error creating link for {} ({})", display_name, e).into()
                    )
                } else {
                    self.log(
                        format!("Created link for {}", display_name)
                    );
                    self.successes += 1;
                }

            } else {
                if let Err(e) = std::fs::remove_file(linkname) {
                    self.log(
                        format!("Error removing link for {} ({})", display_name, e).into()
                    )
                } else {
                    self.log(
                        format!("Deleted link for {}", display_name)
                    );
                    self.successes += 1;
                }
            }
        }
    }

    pub fn scan_and_process_apps(&mut self) {
        let steamdirs = steamlocate::locate_all().expect("Could not find steamapps");

        for steam_dir in steamdirs {
            steam_dir.libraries().unwrap()
                .filter_map(Result::ok)
                .for_each(|library| {
                    self.log(format!("Found library {}", library.path().to_str().unwrap()));
                    library
                        .apps()
                        .filter_map(Result::ok)
                        .for_each(|a|
                            self.process_app(&library, &a)
                        );
                })
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