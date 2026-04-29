mod linker;

use eframe::egui;
use eframe::egui::{Button, Color32, Label, Layout, RichText, TextWrapMode};
use std::sync::mpsc;
use std::thread::JoinHandle;

fn create_links(log_send: mpsc::Sender<String>) -> JoinHandle<()> {
    std::thread::spawn(|| {
        let mut linker = linker::Linker::new(log_send, false);
        linker.scan_and_process_apps();
    })
}

fn delete_links(log_send: mpsc::Sender<String>) -> JoinHandle<()> {
    std::thread::spawn(|| {
        let mut linker = linker::Linker::new(log_send, true);
        linker.scan_and_process_apps();
    })
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 360.0]),
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
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.with_layout(Layout::top_down(egui::Align::TOP).with_cross_justify(false), |ui| {
                match self.screen {
                    AppScreen::Menu => {
                        ui.with_layout(Layout::left_to_right(egui::Align::TOP), |ui2| {
                            if ui2.add_sized([ui2.available_size().x / 2.0, 35.0], Button::new("Create links")).clicked() {
                                self.log.clear();
                                self.screen = AppScreen::Progress;
                                self.progress_message = "Creating links".into();
                                self.pending_action = Some(
                                    create_links(self.log_send.clone())
                                );
                            }
                            if ui2.add_sized([ui2.available_size().x, 35.0], Button::new("Delete links")).clicked() {
                                self.log.clear();
                                self.screen = AppScreen::Progress;
                                self.progress_message = "Deleting links".into();
                                self.pending_action = Some(
                                    delete_links(self.log_send.clone())
                                );
                            }
                        });
                    },
                    AppScreen::Progress => {
                        ui.label(&self.progress_message);
                    }
                }

                ui.separator();
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    for log in &self.log {
                    ui.add(
                            if (log.starts_with("Error")) {
                               Label::new(RichText::new(log).color(Color32::RED))
                            } else {
                                Label::new(log).wrap_mode(TextWrapMode::Wrap)
                            }
                        );
                    }
                    if scroll_to_end {
                        ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                    }
                });
            });
        });
    }
}