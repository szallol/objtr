use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use anyhow::{Context, Result};
use eframe::egui;
use eframe::egui::ahash::HashMap;
use log::debug;
use walkdir::WalkDir;

type Progress = (String, f32);

fn main() -> Result<()> {
    let (tx_process, rx_process) = mpsc::channel::<String>();
    let (tx_progress, rx_progress) = mpsc::channel::<Progress>();

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(false),
        ..Default::default()
    };

    thread::spawn(move || {
        while let Ok(received) = rx_process.recv() {
            debug!("got .obj to process: {}", received);
            let _ = tx_progress.send((received, 12.5));
        }
        // let lines = read_lines("c:/work/_help/_sunix/terra_obj/BlockBABY/BlockBABY.obj")?;
        // for line in lines.flatten() {

        //     match &line[..2] {
        //         "v " => {
        //             println!("{}", translate_vertex(&line)?)
        //         }
        //         _ => println!("{}", line),
        //     }
        // }
    });

    let _ = eframe::run_native(
        ".obj vertex translator",
        options,
        Box::new(|_cc| {
            Ok(Box::new(ConvertApp {
                tx: tx_process,
                rx_progress,
                obj_source_path: None,
                obj_files: None,
                conver_enabled: false,
            }))
        }),
    );

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
    checked: bool,
    progress: f32,
}

struct ConvertApp {
    tx: mpsc::Sender<String>, // path of the .obj
    rx_progress: mpsc::Receiver<Progress>,
    obj_source_path: Option<String>,
    obj_files: Option<HashMap<String, ObjInfo>>,
    conver_enabled: bool,
    // dest_path: Option<String>,
}

impl eframe::App for ConvertApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(progress) = self.rx_progress.try_recv() {
            if let Some(obj_files) = &mut self.obj_files {
                if let Some(obj_file) = obj_files.get_mut(&progress.0) {
                    obj_file.progress = progress.1;
                }
            }
        }

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
                        obj_files.iter_mut().for_each(|(path, obj_info)| {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut obj_info.checked, path);
                                if obj_info.progress > 0.0 {
                                    let progress_bar =
                                        egui::ProgressBar::new(obj_info.progress).show_percentage();
                                    ui.add(progress_bar);
                                }
                            });
                        });
                    }
                });
            };

            if self.obj_files.is_some() {
                ui.add_enabled_ui(self.conver_enabled, |ui| {
                    if ui.button("Convert..").clicked() {
                        if let Some(obj_files) = &self.obj_files {
                            obj_files.iter().for_each(|(path, _obj_info)| {
                                let _ = self.tx.send(path.clone());
                            });
                        }
                        // self.tx.send("csecs".to_owned()).unwrap();
                        debug!("convert clicked");
                        self.conver_enabled = false;
                    }
                });
            }
        });
    }
}

fn all_obj_files_recursively(search_path: &str) -> Option<HashMap<String, ObjInfo>> {
    Some(
        WalkDir::new(search_path)
            .into_iter()
            .filter(|e| {
                // debug!("direntry: {:?}", e.as_ref().unwrap().path().extension());
                e.as_ref().unwrap().path().extension() == Some(OsStr::new("obj"))
            })
            .map(|e| {
                (
                    e.unwrap().into_path().display().to_string(),
                    ObjInfo {
                        checked: true,
                        progress: 0.0,
                    },
                )
            })
            .collect(),
    )
}
