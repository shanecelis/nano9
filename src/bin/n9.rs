use bevy::{
    asset::{
        io::{AssetSourceBuilder, AssetSourceId},
        AssetPath,
    },
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin},
    prelude::*,
    text::FontSmoothing,
};
#[cfg(feature = "minibuffer")]
use bevy_minibuffer::prelude::*;
use clap::{Parser, Subcommand};
use nano9::{
    config::{front_matter, run_pico8_when_loaded, Config},
    pico8::{Pico8Asset, Pico8Handle, PICO8_FONT},
    *,
};
use std::{env, fs, io, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(version, about, long_about, disable_help_subcommand = true,
          // subcommand_required = true,
          // arg_required_else_help = true,
)]
// #[command(next_line_help = true)]
struct Cli {
    // /// Optional name to operate on
    // name: Option<String>,

    // Sets a custom config file
    // #[arg(short, long, value_name = "FILE")]
    // config: Option<PathBuf>,

    // /// Turn debugging information on
    // #[arg(short, long, action = clap::ArgAction::Count)]
    // debug: u8,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a Pico-8 cart or Nano-9 project
    Run {
        /// Run path.
        path: PathBuf,
    },
    /// Create a new Nano-9 project
    ///
    /// Depending on the extension and arguments, it will create the following kind of project:
    ///   - FILE.p8lua, a one file Pico-8 Lua game with embedded config
    ///   - FILE.lua, a one file Lua game with embedded config
    ///   - [--language lua] FILE, a directory with config and Lua code (no Rust)
    ///   - --language rust FILE, a Rust-only crate with config in assets directory
    ///   - --language lua-rust FILE, a Rust crate with config and Lua code in assets directory
    #[command(verbatim_doc_comment)]
    New {
        /// Language
        // #[arg(long, default_value = "lua")]
        #[arg(long)]
        language: Option<Language>,
        /// Choose a starter template
        #[arg(long)]
        starter: Option<StarterKit>,
        /// Overwrite files if present
        #[arg(long)]
        force: bool,
        /// Destination path
        ///
        /// A one-file project is created if the path ends with the following
        /// extensions: .p8lua and .lua.
        path: PathBuf,
    },
    /// Report on available features
    Info {},
}

#[derive(Debug, Clone, clap::ValueEnum, PartialEq)]
enum Language {
    Rust,
    Lua,
    LuaRust,
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum StarterKit {
    Platformer,
    TopDown,
}
// Secondary command line interface used as fallback.
//
// [source](https://stackoverflow.com/a/79564853/6454690)
#[derive(Parser)]
#[command(long_about = None)]
struct CliDefault {
    path: PathBuf,
}

fn main() -> io::Result<ExitCode> {
    let cli = match Cli::try_parse().or_else(|err| match err.kind() {
        clap::error::ErrorKind::InvalidSubcommand => CliDefault::try_parse()
            .map(|cli_default| Cli {
                command: Command::Run {
                    path: cli_default.path,
                },
            })
            .map_err(|_| err),
        _ => Err(err),
    }) {
        Ok(cli) => cli,
        Err(err) => {
            err.print().expect("error writing usage");
            // this will print any errors the same way as Cli::parse() would
            return Ok(ExitCode::from(err.exit_code() as u8));
        }
    };
    // let cli = Cli::parse();
    // let example_files = [
    //     "cart.p8",
    //     "cart.p8.png",
    //     "code.lua", // Lua
    //     // "code.pua", // Pico-8 dialect
    //     "game-dir",
    //     "game-dir/Nano9.toml",
    //     "code.n9",
    // ];
    match cli.command {
        Command::Run { .. } => run(cli),
        Command::New { .. } => new(cli),
        Command::Info { .. } => info(cli),
    }
}

fn info(_cli: Cli) -> io::Result<ExitCode> {
    macro_rules! feature_info {
        ($feature:literal, $description:literal, $enabled_by_default:expr) => {{
            let mark: char = match (cfg!(feature = $feature), $enabled_by_default) {
                (true, true) => 'x',
                (false, true) => '_',
                (true, false) => 'X',
                (false, false) => ' ',
            };
            println!("  - [{}] {:?} {}", mark, $feature, $description);
        }};
    }
    println!(
        r#"The following features are available. Use this key:
                                    - [x] enabled and enabled by default
                                    - [X] enabled and disabled by default
                                    - [_] disabled and enabled by default
                                    - [ ] disabled and disabled by default"#
    );
    feature_info!("scripting", "for Lua scripting", true);
    feature_info!("negate-y", "uses Pico-8's positive-y is downward", true);
    feature_info!("pixel-snap", "applies floor to pixel locations", true);
    feature_info!("pico8-to-lua", "converts Pico-8's dialect to Lua", true);
    feature_info!("fixed-point", "uses fixed-point numbers for bit operations", true);
    feature_info!("web-asset", "allows URLs for asset locations", false);
    feature_info!("minibuffer", "embeds a gamedev console", false);
    feature_info!("inspector", "adds inspector commands to console", false);
    feature_info!("cmd_lib", "run commands for 'n9 new'", true);
    feature_info!("clap", "argument parsing for 'n9'", true);
    Ok(ExitCode::from(0))
}

fn new(cli: Cli) -> io::Result<ExitCode> {
    match cli.command {
        Command::New {
            language,
            starter,
            path,
            force,
        } => {
            use log::info;
            env_logger::Builder::from_env(
                env_logger::Env::default().default_filter_or("warn,n9=info"),
            )
            .init();
            if let Some(_starter) = starter {
                todo!();
            } else if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
                match extension {
                    "lua" => {
                        // Copy the lua template.
                        let content = include_str!("../../examples/line.lua");
                        fs::write(path, content)?;
                        Ok(ExitCode::from(0))
                    }
                    "p8lua" => {
                        // Copy the p8lua template.
                        let content = include_str!("../../examples/line.p8lua");
                        fs::write(path, content)?;
                        Ok(ExitCode::from(0))
                    }
                    ext => {
                        eprintln!("error: No template for extension {ext:?}.");
                        Ok(ExitCode::from(5))
                    }
                }
            } else {
                // It's a directory path.
                match language {
                    lang @ Some(Language::Rust | Language::LuaRust) => {
                        #[cfg(feature = "cmd_lib")]
                        {
                            use cmd_lib::run_cmd;
                            info!("Creating new cargo project at {:?}.", &path);
                            Ok(
                                match run_cmd!(cargo new $path;
                                               cd $path;
                                               cargo add bevy@0.15;
                                               // cargo add nano9 --features lib --no-default-features
                                               // cargo add nano9;
                                               cargo add --path ..;
                                ) {
                                    Ok(_) => {
                                        // Copy files
                                        let content = include_str!("templates/Nano9.toml");
                                        let mut p = path.to_path_buf();
                                        p.push("assets");
                                        fs::create_dir_all(&p)?;
                                        p.push("Nano9.toml");
                                        info!("Creating Nano-9 config at {:?}.", &p);
                                        fs::write(&p, content)?;

                                        if lang == Some(Language::LuaRust) {
                                            let content = include_str!("templates/main.lua");
                                            let _ = p.pop();
                                            p.push("main.lua");
                                            info!("Creating main Lua code at {:?}.", &p);
                                            fs::write(&p, content)?;

                                            let content = include_str!("templates/main-lua-rust.rs.txt");
                                            let _ = p.pop();
                                            let _ = p.pop();
                                            p.push("src/main.rs");
                                            info!("Creating main Rust code at {:?}.", &p);
                                            fs::write(&p, content)?;
                                        } else {
                                            let content = include_str!("templates/main-rust.rs.txt");
                                            let _ = p.pop();
                                            let _ = p.pop();
                                            p.push("src/main.rs");
                                            info!("Creating main Rust code at {:?}.", &p);
                                            fs::write(&p, content)?;
                                        }

                                        ExitCode::from(0)
                                    }
                                    Err(e) => {
                                        eprintln!("error: Problem running cargo {e}");
                                        ExitCode::from(8)
                                    }
                                },
                            )
                        }
                        #[cfg(not(feature = "cmd_lib"))]
                        {
                            eprintln!(
                                "error: Cannot create new crate when {:?} feature is disabled.",
                                "cmd_lib"
                            );
                            Ok(ExitCode::from(9))
                        }

                    }
                    Some(Language::Lua) | None => {
                        if path.exists() {
                            if path.is_file() {
                                eprintln!("error: {path:?} is a file; a directory was expected.");
                                return Ok(ExitCode::from(6));
                            }
                            if path.is_dir() && !force {
                                eprintln!("error: {path:?} already exists, canceling; cautiously use --force to overwrite.");
                                return Ok(ExitCode::from(7));
                            }
                        } else {
                            fs::create_dir_all(&path)?;
                        }
                        let config = include_str!("../../examples/sprite/Nano9.toml");
                        let mut p = path.to_path_buf();
                        p.push("Nano9.toml");
                        fs::write(&p, config)?;

                        let code = include_str!("../../examples/sprite/main.p8lua");
                        let mut code_path = path.to_path_buf();
                        code_path.push("main.lua");
                        fs::write(&code_path, code)?;
                        Ok(ExitCode::from(0))
                    }
                }
            }
        }
        _ => unreachable!(),
    }
}

fn run(cli: Cli) -> io::Result<ExitCode> {
    let script = match cli.command {
        Command::Run { path } => path,
        _ => unreachable!(),
    };
    let script_path = {
        let mut path = PathBuf::from(&script);
        if path.is_dir() {
            path.push("Nano9.toml")
        }
        path
    };
    let mut app = App::new();
    let cwd = AssetSourceId::Name("cwd".into());
    let mut builder = AssetSourceBuilder::platform_default(
        dbg!(env::current_dir()?.to_str().expect("current dir")),
        None,
    );
    builder.watcher = None;
    builder.processed_watcher = None;
    app.register_asset_source(&cwd, builder);

    let set_default_source = if let Some(dir_name) = env::var_os("NANO9_ASSETS_DIR") {
        let mut asset_dir: PathBuf = dir_name.into();
        if asset_dir.is_relative() {
            let mut cur_dir = env::current_dir()?;
            cur_dir.push(&asset_dir);
            asset_dir = cur_dir;
        }
        app.register_asset_source(
            &AssetSourceId::Default,
            AssetSourceBuilder::platform_default(asset_dir.to_str().expect("asset dir"), None),
        );
        true
    } else {
        false
    };

    let nano9_plugin;

    let extension = script_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    match extension {
        "toml" => {
            eprintln!("loading config");
            let path = &script_path;
            if set_default_source {
                eprintln!("warn: NANO9_ASSETS_DIR environment variable overriding Nano-9.toml's directory.");
            } else if let Some(parent) = path.parent() {
                app.register_asset_source(
                    &AssetSourceId::Default,
                    AssetSourceBuilder::platform_default(
                        parent.to_str().expect("parent dir"),
                        None,
                    ),
                );
            }
            // OLD SHANE: Get rid of this.
            //
            // NEW SHANE: No. We use part of Config to configure the App and can't
            // do that at load time.
            let content = fs::read_to_string(path)?;
            let mut config: Config = toml::from_str::<Config>(&content)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;

            if let Err(e) = config.inject_template(None) {
                eprintln!("error: {e}");
                return Ok(ExitCode::from(2));
            }
            nano9_plugin = Nano9Plugin { config };
        }
        "p8" | "png" => {
            eprintln!("loading cart");
            let config = Config::pico8();

            // let asset_path = AssetPath::from_path(&script_path).into_owned().with_source(&cwd).with_label("lua");
            // config.code = Some(asset_path.to_string());
            let path = script_path.clone();
            app.add_systems(
                Startup,
                move |asset_server: Res<AssetServer>, mut commands: Commands| {
                    let asset_path = AssetPath::from_path(&path).with_source(&cwd);
                    let pico8_asset: Handle<Pico8Asset> = asset_server.load(&asset_path);
                    commands.insert_resource(Pico8Handle::from(pico8_asset));
                },
            );
            nano9_plugin = Nano9Plugin { config };
        }
        "lua" | "p8lua" => {
            if cfg!(not(feature = "pico8-to-lua")) && extension == "p8lua" {
                eprintln!(
                    "error: Must compile with 'pico8-to-lua' feature to handle 'p8lua' files."
                );
                return Ok(ExitCode::from(3));
            }
            eprintln!("loading lua");
            let mut content = fs::read_to_string(&script_path)?;

            let mut config =
                if let Some(front_matter) = front_matter::LUA.parse_in_place(&mut content) {
                    let mut config: Config = toml::from_str::<Config>(&front_matter)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                    config
                        .inject_template(None)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e}")))?;
                    config
                } else {
                    Config::pico8()
                };
            config.code = Some(dbg!(AssetPath::from_path(&script_path)
                .with_source(&cwd)
                .to_string()));
            nano9_plugin = Nano9Plugin { config };
        }
        _ext => {
            eprintln!("Only accepts .p8, .png, .lua, .p8lua, and .toml files.");
            return Ok(ExitCode::from(1));
        }
    }

    app.add_plugins(Nano9Plugins {
        config: nano9_plugin.config,
    });

    let font: Handle<Font> = {
        if let Some(asset_server) = app.world().get_resource::<AssetServer>() {
            asset_server.load(PICO8_FONT)
        } else {
            default()
        }
    };
    app.add_plugins(FpsOverlayPlugin {
        config: FpsOverlayConfig {
            text_config: TextFont {
                // Here we define size of our overlay
                font_size: 24.0,
                // If we want, we can use a custom font
                font,
                // We could also disable font smoothing,
                font_smoothing: FontSmoothing::None,
            },
            // We can also change color of the overlay
            text_color: Color::WHITE,
            enabled: false,
        },
    })
    .add_systems(PreUpdate, run_pico8_when_loaded);

    #[cfg(feature = "minibuffer")]
    app.add_plugins(MinibufferPlugins).add_acts((
        BasicActs::default(),
        // acts::universal::UniversalArgActs::default(),
        // acts::tape::TapeActs::default(),
        crate::minibuffer::Nano9Acts::default(),
        // CountComponentsActs::default()
        //     .add::<Text>("text")
        //     .add::<TilemapType>("map")
        //     .add::<TilePos>("tile")
        //     .add::<Sprite>("sprite")
        //     .add::<Clearable>("clearables"),
        Act::new(toggle_fps).bind(keyseq! { Space N F }), // inspector::AssetActs::default().add::<Image>(),
    ));

    #[cfg(all(feature = "minibuffer", feature = "inspector"))]
    app.add_acts((
        bevy_minibuffer_inspector::WorldActs::default(),
        bevy_minibuffer_inspector::StateActs::default().add::<crate::error::RunState>(),
    ));
    #[cfg(all(feature = "level", feature = "user_properties"))]
    app.add_systems(Startup, |reg: Res<AppTypeRegistry>| {
        bevy_ecs_tiled::map::export_types(&reg, "all-export-types.json", |name| true);
        bevy_ecs_tiled::map::export_types(&reg, "export-types.json", |name| {
            name.contains("bevy_ecs_tilemap::tiles") || name.contains("nano9")
        });
    });
    app.run();

    Ok(ExitCode::from(0))
}

#[cfg(feature = "minibuffer")]
fn toggle_fps(mut config: ResMut<FpsOverlayConfig>) {
    config.enabled = !config.enabled;
}
