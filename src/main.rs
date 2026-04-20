mod linker;

use steamlocate;
use eframe::egui;
use tokio::task::JoinHandle;
use std::sync::mpsc;
use eframe::egui::{Layout, TextEdit};

async fn create_links(log_send: mpsc::Sender<String>) {
    let mut linker = linker::Linker::new(log_send, false);
    linker.scan_and_process_apps();
}

async fn delete_links(log_send: mpsc::Sender<String>) {
    let mut linker = linker::Linker::new(log_send, true);
    linker.scan_and_process_apps();
}

#[tokio::main]
async fn main() -> eframe::Result {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ProtonShortcut",
        options,
        Box::new(|cc| {
            Ok(Box::<MyApp>::default())
        }),
    )
}

enum AppScreen {
    Menu,
    Progress
}

struct MyApp {
    screen: AppScreen,
    progress_message: String,
    pending_action: Option<JoinHandle<()>>,
    log_recv: mpsc::Receiver<String>,
    log_send: mpsc::Sender<String>,
    log: Vec<String>,
    log_rendered: String
}

impl Default for MyApp {
    fn default() -> Self {
        let (log_send, log_recv) = mpsc::channel();

        Self {
            screen: AppScreen::Menu,
            pending_action: None,
            progress_message: String::new(),
            log_send,
            log_recv,
            log: Vec::new(),
            log_rendered: String::new()
        }
    }
}

impl eframe::App for MyApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut scroll_to_end = false;
        if let Some(future) = &self.pending_action {
            if future.is_finished() {
                self.screen = AppScreen::Menu
            }
        }

        if let Ok(m) = self.log_recv.try_recv() {
            println!("Recieved {}", m);
            scroll_to_end = true;
            self.log.push(m);
            self.log_rendered = self.log.join("\n");
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.with_layout(Layout::top_down(egui::Align::TOP).with_cross_justify(true), |ui| {
                match self.screen {
                    AppScreen::Menu => {
                        if ui.button("Create links").clicked() {
                            self.screen = AppScreen::Progress;
                            self.progress_message = "Creating links".into();
                            self.pending_action = Some(
                                tokio::spawn(create_links(self.log_send.clone()))
                            );
                        }
                        if ui.button("Delete links").clicked() {
                            self.screen = AppScreen::Progress;
                            self.progress_message = "Deleting links".into();
                            self.pending_action = Some(tokio::spawn(delete_links(self.log_send.clone())));
                        }
                    },
                    AppScreen::Progress => {
                        ui.label(&self.progress_message);
                    }
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add_sized(
                        ui.available_size(),
                        TextEdit::multiline(&mut self.log_rendered).cursor_at_end(true)
                    );
                    if scroll_to_end {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
            });
        });
    }
}