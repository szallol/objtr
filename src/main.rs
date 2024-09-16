use anyhow::{Context, Result};
use eframe::egui;
use log::debug;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use walkdir::WalkDir;

fn main() -> Result<()> {
    // let lines = read_lines("c:/work/_help/_sunix/terra_obj/BlockBABY/BlockBABY.obj")?;

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 240.0]) // wide enough for the drag-drop overlay text
            .with_drag_and_drop(false),
        ..Default::default()
    };
    let _ = eframe::run_native(
        ".obj vertex translator",
        options,
        Box::new(|_cc| Ok(Box::<MyApp>::default())),
    );

    // for line in lines.flatten() {
    //     match &line[..2] {
    //         "v " => {
    //             println!("{}", translate_vertex(&line)?)
    //         }
    //         _ => println!("{}", line),
    //     }
    // }

    Ok(())
}

fn translate_vertex(vstr: &str) -> Result<String> {
    let mut cols = vstr
        .split_ascii_whitespace()
        .skip(1)
        .map(|s| s.parse::<f32>().unwrap());

    let x = cols
        .next()
        .with_context(|| format!("failed to parse X in line: {}", vstr))?
        + 498521.12878285768;
    let y = cols
        .next()
        .with_context(|| format!("failed to parse Y in line: {}", vstr))?
        + 389878.04510264623;
    let z = cols
        .next()
        .with_context(|| format!("failed to parse Z in line: {}", vstr))?
        + 545.18800000022418;
    Ok(format!("v {} {} {}", x, y, z))
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

#[derive(Default)]
struct ObjInfo {
    path: String,
    checked: bool,
}

#[derive(Default)]
struct MyApp {
    obj_source_path: Option<String>,
    obj_files: Option<Vec<ObjInfo>>,
    conver_enabled: bool,
    // dest_path: Option<String>,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label("Select folder with Block directories and metadata.xml");

            if ui.button("Select directory…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.obj_source_path = Some(path.display().to_string());
                    self.obj_files = all_obj_files_recursively(&path.display().to_string());
                    self.conver_enabled = true;
                }
            }

            if let Some(picked_path) = &self.obj_source_path {
                ui.horizontal(|ui| {
                    ui.label("Selected directory:");
                    ui.monospace(picked_path);
                });

                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(obj_files) = &mut self.obj_files {
                        obj_files.iter_mut().for_each(|obj_info| {
                            ui.checkbox(&mut obj_info.checked, obj_info.path.clone());
                        })
                    }
                });
            };

            if self.obj_files.is_some() {
                ui.add_enabled_ui(self.conver_enabled, |ui| {
                    if ui.button("Convert..").clicked() {
                        debug!("convert clicked");
                        self.conver_enabled = false;
                    }
                });
            }
        });
    }
}

fn all_obj_files_recursively(search_path: &str) -> Option<Vec<ObjInfo>> {
    Some(
        WalkDir::new(search_path)
            .into_iter()
            .filter(|e| {
                // debug!("direntry: {:?}", e.as_ref().unwrap().path().extension());
                e.as_ref().unwrap().path().extension() == Some(OsStr::new("obj"))
            })
            .map(|e| ObjInfo {
                path: e.unwrap().into_path().display().to_string(),
                checked: true,
            })
            .collect(),
    )
}
