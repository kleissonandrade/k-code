#[allow(dead_code)]
mod action;
mod app;
mod components;
mod config;
#[allow(dead_code)]
mod error;
mod event;
mod icons;
mod keymap;
mod layout;
mod mode;
mod terminal;
#[allow(dead_code)]
mod theme;

use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;

use app::App;
use event::EventHandler;

#[derive(Parser, Debug)]
#[command(name = "k-code", version, about = "A fast terminal code editor")]
struct Cli {
    /// Path to open (file or directory)
    #[arg(default_value = ".")]
    path: String,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    // Setup logging to file
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("k-code");
    let _ = std::fs::create_dir_all(&log_dir);
    let file_appender = tracing_appender::rolling::daily(&log_dir, "k-code.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    let cli = Cli::parse();
    let input_path = PathBuf::from(&cli.path);

    let (workspace_root, file_to_open) = if input_path.is_file() {
        let parent = input_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();
        (
            std::fs::canonicalize(&parent)?,
            Some(std::fs::canonicalize(&input_path)?),
        )
    } else if input_path.is_dir() {
        (std::fs::canonicalize(&input_path)?, None)
    } else {
        (std::env::current_dir()?, None)
    };

    let mut tui = terminal::init()?;
    let mut events = EventHandler::new(Duration::from_millis(16));
    let mut app = App::new(workspace_root);

    if let Some(file_path) = file_to_open {
        app.open_file(file_path);
    }

    while app.running {
        tui.draw(|frame| app.render(frame))?;

        if let Some(event) = events.next().await {
            let action = app.handle_event(event);
            app.process_action(action);
        }
    }

    terminal::restore()?;
    Ok(())
}
