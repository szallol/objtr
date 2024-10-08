use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::sync::mpsc;
use std::thread;

use anyhow::{Context, Result};
use eframe::egui;
use eframe::egui::ahash::HashMap;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_xml_rs::from_str;
use walkdir::WalkDir;

type Progress = (String, f32);

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ModelMetadata {
    #[serde(rename = "SRSOrigin")]
    srsorigin: String,
}

#[derive(Clone, Debug)]
struct Srs {
    x: f64,
    y: f64,
    z: f64,
}

fn main() -> Result<()> {
    let (tx_process, rx_process) = mpsc::channel::<(String, Srs)>();
    let (tx_progress, rx_progress) = mpsc::channel::<Progress>();

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_drag_and_drop(false),
        ..Default::default()
    };

    thread::spawn(move || {
        while let Ok((received_file, srs)) = rx_process.recv() {
            let (lines, file_size) = read_lines(&received_file).unwrap();
            debug!("got .obj to process: {} / {}", received_file, file_size);

            let input_path = Path::new(&received_file);
            let output_path = input_path.parent().unwrap().join(format!(
                "{}_tr.obj",
                input_path.file_stem().unwrap().to_str().unwrap()
            ));

            let mut output = File::create(output_path).unwrap();

            let mut progress_size: usize = 0;

            for (index, line) in lines.map_while(Result::ok).enumerate() {
                progress_size += line.len() + 1;

                if index % 1_000_000 == 0 {
                    let progress = progress_size as f64 / file_size as f64;
                    let _ = tx_progress.send((received_file.clone(), progress as f32));
                }

                match &line[..2] {
                    "v " => {
                        let _ = writeln!(output, "{}", translate_vertex(&line, &srs).unwrap());
                    }
                    _ => {
                        let _ = writeln!(output, "{}", line);
                    }
                }
            }
            let _ = tx_progress.send((received_file.clone(), 1.0)); // %100
        }
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
                srs: None,
            }))
        }),
    );

    Ok(())
}

fn read_lines<P>(filename: P) -> io::Result<(io::Lines<io::BufReader<File>>, usize)>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    let file_size = file.metadata().unwrap().len();
    Ok((io::BufReader::new(file).lines(), file_size as usize))
}

fn translate_vertex(vstr: &str, srs: &Srs) -> Result<String> {
    let mut cols = vstr
        .split_ascii_whitespace()
        .skip(1)
        .map(|s| s.parse::<f64>().unwrap());

    let x = cols
        .next()
        .with_context(|| format!("failed to parse X in line: {}", vstr))?
        + srs.x;
    let y = cols
        .next()
        .with_context(|| format!("failed to parse Y in line: {}", vstr))?
        + srs.y;
    let z = cols
        .next()
        .with_context(|| format!("failed to parse Z in line: {}", vstr))?
        + srs.z;
    Ok(format!("v {} {} {}", x, y, z))
}

#[derive(Default)]
struct ObjInfo {
    checked: bool,
    progress: f32,
}

struct ConvertApp {
    tx: mpsc::Sender<(String, Srs)>, // path of the .obj
    rx_progress: mpsc::Receiver<Progress>,
    obj_source_path: Option<String>,
    obj_files: Option<HashMap<String, ObjInfo>>,
    conver_enabled: bool,
    srs: Option<Srs>,
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
            ui.ctx().request_repaint_after_secs(0.2); // to update progress if no event generated

            ui.label("Select folder with Block directories and metadata.xml");

            if ui.button("Select directory…").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    let metadata_file = path.join("metadata.xml");
                    if let Ok(srs_data) = fs::read_to_string(metadata_file) {
                        let metadata = from_str::<ModelMetadata>(&srs_data).unwrap();
                        debug!("found srs metadata: {:?}", metadata);

                        let origins: Vec<f64> = metadata
                            .srsorigin
                            .split(",")
                            .map(|value| value.parse::<f64>().unwrap())
                            .collect();
                        self.srs = Some(Srs {
                            x: origins[0],
                            y: origins[1],
                            z: origins[2],
                        });

                        self.obj_source_path = Some(path.display().to_string());
                        self.obj_files = all_obj_files_recursively(&path.display().to_string());
                        self.conver_enabled = true;
                    }
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
                            if let Some(srs) = &self.srs {
                                obj_files.iter().for_each(|(path, _obj_info)| {
                                    let _ = self.tx.send((path.clone(), srs.clone()));
                                });
                            }
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
