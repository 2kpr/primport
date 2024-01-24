use super::{GameVersion, PrimPort};
use eframe::egui;
use egui::{FontFamily, FontId, TextStyle};
use std::path::PathBuf;

#[derive(Default)]
pub struct MyApp {
    prim_port: PrimPort,
    input_prim_path: String,
    output_prim_path: String,
    input_version: usize,
    output_version: usize,
    game_versions: Vec<String>,
    window_open: bool,
}

impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>, prim_port: PrimPort) -> Self {
        configure_text_styles(&cc.egui_ctx);
        let mut game_versions = Vec::new();
        game_versions.push("HMA".to_string());
        game_versions.push("ALPHA".to_string());
        game_versions.push("HM2016".to_string());
        game_versions.push("WOA".to_string());
        Self {
            prim_port: prim_port,
            input_prim_path: String::new(),
            output_prim_path: String::new(),
            input_version: 3,
            output_version: 3,
            game_versions: game_versions,
            window_open: false,
        }
    }
}

fn configure_text_styles(ctx: &egui::Context) {
    use FontFamily::{Monospace, Proportional};

    let mut style = (*ctx.style()).clone();
    style.text_styles = [
        (TextStyle::Heading, FontId::new(42.0, Proportional)),
        (
            TextStyle::Name("Heading2".into()),
            FontId::new(22.0, Proportional),
        ),
        (
            TextStyle::Name("ContextHeading".into()),
            FontId::new(19.0, Proportional),
        ),
        (TextStyle::Body, FontId::new(16.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
        (TextStyle::Button, FontId::new(16.0, Proportional)),
        (TextStyle::Small, FontId::new(8.0, Proportional)),
    ]
    .into();
    ctx.set_style(style);
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.window_open {
            egui::Window::new("")
                .title_bar(false)
                .fixed_pos((150.0, 130.0))
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label("PRIM Successfully Ported!");
                        ui.add_space(20.0);
                        let button = ui.add_sized([60.0, 30.0], egui::Button::new("Ok"));
                        if button.clicked() {
                            self.window_open = false;
                        }
                    });
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.set_enabled(!self.window_open);
            ui.vertical_centered(|ui| {
                ui.heading("PrimPort");
                ui.label("Select input and output files and click Port!");
                ui.label("For CLI Usage: primport.exe <input_prim> <input_version> <output_version> <output_prim>");
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("Select Input PRIM file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.input_prim_path = path.display().to_string();
                    }
                }
                ui.add_space(224.0);
                ui.label("Game Version:");
                egui::ComboBox::from_id_source("Input Game Version")
                    .selected_text(format!("{}", &self.game_versions[self.input_version]))
                    .show_ui(ui, |ui| {
                        for i in 0..self.game_versions.len() {
                            let value = ui.selectable_value(
                                &mut &self.game_versions[i],
                                &self.game_versions[self.input_version],
                                &self.game_versions[i],
                            );
                            if value.clicked() {
                                self.input_version = i;
                            }
                        }
                    });
            });
            ui.label("Input PRIM file:");
            ui.add_sized(
                [640.0, 0.0],
                egui::TextEdit::singleline(&mut self.input_prim_path),
            );

            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button("Select Output PRIM file…").clicked() {
                    if let Some(path) = rfd::FileDialog::new().save_file() {
                        self.output_prim_path = path.display().to_string();
                    }
                }
                ui.add_space(210.0);
                ui.label("Game Version:");
                egui::ComboBox::from_id_source("Output Game Version")
                    .selected_text(format!("{}", &self.game_versions[self.output_version]))
                    .show_ui(ui, |ui| {
                        for i in 0..self.game_versions.len() {
                            let value = ui.selectable_value(
                                &mut &self.game_versions[i],
                                &self.game_versions[self.output_version],
                                &self.game_versions[i],
                            );
                            if value.clicked() {
                                self.output_version = i;
                            }
                        }
                    });
            });
            ui.label("Output PRIM file:");
            ui.add_sized(
                [640.0, 0.0],
                egui::TextEdit::singleline(&mut self.output_prim_path),
            );

            ui.separator();

            ui.vertical_centered(|ui| {
                let button = ui.add_sized([100.0, 40.0], egui::Button::new("Port!"));
                if button.clicked() {
                    self.prim_port.input_prim_path = PathBuf::from(&self.input_prim_path);
                    self.prim_port.output_prim_path = PathBuf::from(&self.output_prim_path);
                    self.prim_port.input_version = GameVersion::try_from(self.input_version as u8).unwrap();
                    self.prim_port.output_version = GameVersion::try_from(self.output_version as u8).unwrap();
                    self.prim_port.port();
                    self.window_open = true;
                }
            });
        });
    }
}
