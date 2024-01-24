mod gui;
mod io;
mod prim;
use clap::Parser;
use eframe::egui;
use prim::Prim;
use std::path::PathBuf;

#[derive(Parser, Default)]
struct Cli {
    /// Remove cloth meshes (When porting from ALPHA)
    #[arg(short = 'c')]
    no_cloth: bool,
    /// Enable verbose debug output
    #[arg(short = 'v')]
    verbose: bool,
    /// Path to input PRIM file to port
    input_prim: Option<PathBuf>,
    /// Input PRIM game version: HMA, ALPHA, HM2016, WOA
    input_version: Option<String>,
    /// Output PRIM game version: HMA, ALPHA, HM2016, WOA
    output_version: Option<String>,
    /// Path to output ported PRIM file
    output_prim: Option<PathBuf>,
}

#[derive(Default, Clone, Copy)]
pub enum GameVersion {
    #[default]
    Hma,
    Alpha,
    Hm2016,
    Woa,
}

impl TryFrom<u8> for GameVersion {
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Hma),
            1 => Ok(Self::Alpha),
            2 => Ok(Self::Hm2016),
            3 => Ok(Self::Woa),
            _ => Err(String::from("Error")),
        }
    }
}

impl TryInto<String> for GameVersion {
    type Error = String;
    fn try_into(self) -> Result<String, Self::Error> {
        match self {
            Self::Hma => Ok("HMA".to_string()),
            Self::Alpha => Ok("ALPHA".to_string()),
            Self::Hm2016 => Ok("HM2016".to_string()),
            Self::Woa => Ok("WOA".to_string()),
            _ => Err(String::from("Error")),
        }
    }
}

fn game_version_check(version: &String) -> GameVersion {
    let input_version = match version.to_lowercase().as_str() {
        "hma" =>
        //GameVersion::Hma,
        {
            println!("Error: HMA is not supported yet.");
            std::process::exit(1)
        }
        "alpha" => GameVersion::Alpha,
        "hm2016" => GameVersion::Hm2016,
        "woa" => GameVersion::Woa,
        _ => {
            println!(
                "Error: Input PRIM game version is unknown: {}\nEnter one of the following: HMA, ALPHA, HM2016, WOA",
                version,
            );
            std::process::exit(1);
        }
    };
    input_version
}

#[derive(Default)]
struct PrimPort {
    input_prim_path: PathBuf,
    output_prim_path: PathBuf,
    input_version: GameVersion,
    output_version: GameVersion,
    no_cloth: bool,
    verbose: bool,
    use_gui: bool,
}

impl PrimPort {
    fn from(args: Cli) -> PrimPort {
        if args.input_prim.is_none()
            || args.input_version.is_none()
            || args.output_version.is_none()
            || args.output_prim.is_none()
        {
            PrimPort {
                input_prim_path: PathBuf::new(),
                output_prim_path: PathBuf::new(),
                input_version: GameVersion::Hma,
                output_version: GameVersion::Hma,
                no_cloth: false,
                verbose: false,
                use_gui: true,
            }
        } else {
            PrimPort {
                input_prim_path: args.input_prim.clone().unwrap_or_else(|| PathBuf::new()),
                output_prim_path: args.output_prim.clone().unwrap_or_else(|| PathBuf::new()),
                input_version: game_version_check(&args.input_version.unwrap()),
                output_version: game_version_check(&args.output_version.unwrap()),
                no_cloth: args.no_cloth,
                verbose: args.verbose,
                use_gui: false,
            }
        }
    }

    fn port(&mut self) {
        println!(
            "Porting input PRIM file: {}",
            &self.input_prim_path.to_str().unwrap()
        );
        println!(
            "Porting from game version {} to {}",
            TryInto::<String>::try_into(self.input_version.clone()).unwrap(),
            TryInto::<String>::try_into(self.output_version.clone()).unwrap(),
        );
        println!(
            "Porting to output PRIM file: {}",
            &self.output_prim_path.to_str().unwrap()
        );
        let mut prim = Prim::read(&self.input_prim_path, &self.input_version, self.verbose);
        prim.write(&self.output_prim_path, &self.output_version, self.no_cloth);
        println!("Ported successfully!");
    }
}

fn main() {
    let args = Cli::parse();
    let mut prim_port = PrimPort::from(args);
    if !prim_port.use_gui {
        prim_port.port();
    } else {
        let options = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 340.0]),
            ..Default::default()
        };
        eframe::run_native(
            "PrimPort v0.1.0",
            options,
            Box::new(|_cc| Box::new(gui::MyApp::new(_cc, prim_port))),
        )
        .unwrap();
    }
}
