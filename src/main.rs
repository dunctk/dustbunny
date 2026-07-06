mod app;
mod layout;
mod model;
mod scanner;
mod splash;
mod terminal;
mod ui;

use std::env;
use std::io;
use std::path::PathBuf;

use app::App;
use terminal::{Terminal, read_key, terminal_size};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_help();
        return Ok(());
    }

    let summary_mode = args.iter().any(|arg| arg == "--summary");
    let path = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);

    if summary_mode {
        let tree = scanner::scan_path(&path)?;
        print!("{}", ui::summary(&tree));
        return Ok(());
    }

    let mut terminal = Terminal::enter()?;
    let tree = splash::scan_with_splash(&path, &mut terminal)?;
    let root_path = tree.get(tree.root()).path.clone();
    let mut app = App::new(tree, root_path);

    loop {
        let (width, height) = terminal_size();
        let frame = ui::render(&app, width, height);
        terminal.draw(&frame)?;

        if app.should_quit {
            break;
        }

        let key = read_key()?;
        app.handle_key(key);
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn print_help() {
    println!(
        "dustbunny - terminal disk usage explorer\n\n\
Usage:\n  dustbunny [PATH]\n  dustbunny --summary [PATH]\n\n\
Keys:\n  q/Esc quit, Enter drill in, Backspace parent, arrows select,\n  Tab switch focus, r rescan, o open, ? help"
    );
}
