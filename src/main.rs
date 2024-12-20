use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::fs;
use std::io::stdin;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::{
    env,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use termion::cursor::DetectCursorPos;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;

fn main() {
    let _raw = Some(io::stdout().into_raw_mode().unwrap());

    let mut input = String::new();
    print!("? ");
    io::stdout().flush().unwrap();
    for key in stdin().keys() {
        let key = key.unwrap();
        match key {
            Key::Char('\t') => {
                input = parse_segments(&input);
                print!("\x1b[2K\r");
                print!("? {}", input);
                io::stdout().flush().unwrap();
            }
            Key::Char('\n') => {
                print!("\r\n");
                io::stdout().flush().unwrap();
                if !input.is_empty() && !parse(&input) {
                    return;
                }
                print!("\r");
                print!("? ");
                io::stdout().flush().unwrap();
                input.clear();
            }
            Key::Backspace => {
                if !input.is_empty() {
                    input.pop();
                    print!("\x08 \x08");
                    io::stdout().flush().unwrap();
                }
            }
            Key::Ctrl('c') => {
                print!("\r\n");
                io::stdout().flush().unwrap();
                input.clear();
            }
            Key::Ctrl('l') => {
                print!("\x1B[2J\x1B[H");
                print!("? {}", &input);
                io::stdout().flush().unwrap();
            }
            Key::Char(key) => {
                print!("{}", key);
                io::stdout().flush().unwrap();
                input.push(key);
            }
            _ => {}
        }
    }
}

fn parse_segments(input: &String) -> String {
    let (x, _y) = io::stdout().cursor_pos().unwrap();
    if !input.is_empty() {
        let space_i = match input[..(x as usize - 3)].rfind(' ') {
            Some(i) => i + 1,
            None => {
                return command_autocomplete(&input);
            }
        };
        let seg = &input[space_i..(x as usize - 3)];
        let autocompleted = autocomplete(&seg.to_string());
        let mut input_owned = input.clone();
        input_owned.replace_range(space_i..space_i + seg.len(), &autocompleted);
        return input_owned;
    } else {
        return autocomplete(&input);
    }
}

fn command_autocomplete(input: &String) -> String {
    let mut commands = Vec::new();
    if let Ok(path_var) = env::var("PATH") {
        for split_path in env::split_paths(&path_var) {
            if let Ok(entries) = fs::read_dir(split_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        if entry.file_name().to_str().unwrap().starts_with(input) {
                            commands.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    commands.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    list_files(commands);
    return input.to_owned();
}

fn autocomplete(input: &String) -> String {
    let idx = input.rfind("/").unwrap_or(0);
    let mut search_path = input[..idx].to_owned();
    search_path.insert_str(0, "./");
    let input_path = PathBuf::from(input);
    if input_path.is_dir() {
        if !input.ends_with("/") {
            return input.to_owned() + "/";
        }
    }

    let mut paths: Vec<PathBuf> = fs::read_dir(search_path)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|entry| {
            if input_path.is_dir() {
                return true;
            }
            entry
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with(input_path.file_name().unwrap_or_default().to_str().unwrap())
        })
        .collect();

    if input.is_empty() || input_path.is_dir() {
        list_files(paths);
        return input.to_owned();
    }

    if paths.is_empty() {
        return input.to_owned();
    }

    paths.sort_by_key(|path| {
        return path.to_str().unwrap().len();
    });

    if let Some(found) = paths.iter().find(|x| &x.to_string_lossy() == input) {
        if found.is_dir() {
            return String::from(found.file_name().unwrap().to_str().unwrap()) + "/";
        } else {
            return String::from(found.file_name().unwrap().to_str().unwrap()) + " ";
        }
    }

    let first = &paths[0];
    let mut prefix_len = first.file_name().unwrap().to_str().unwrap().len();
    for s in paths.iter().skip(1) {
        prefix_len = first
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .chars()
            .zip(s.file_name().unwrap().to_str().unwrap().chars())
            .take_while(|(a, b)| a.to_ascii_lowercase() == b.to_ascii_lowercase())
            .count()
            .min(prefix_len)
    }

    if prefix_len == input_path.file_name().unwrap().len() {
        list_files(paths);
    } else {
        return String::from(
            first.to_str().unwrap()[2..first.to_str().unwrap().len()
                - (first.file_name().unwrap().len() - prefix_len)]
                .to_owned(),
        );
    }

    return input.to_owned();
}

fn list_files(paths: Vec<PathBuf>) {
    let file_names: Vec<&std::ffi::OsStr> = paths.iter().map(|p| p.file_name().unwrap()).collect();
    print!("\r\n");
    io::stdout().flush().unwrap();
    let mut longest_file_name_size = file_names
        .iter()
        .map(|name| name.to_str().unwrap().chars().count())
        .max()
        .unwrap_or(0);
    longest_file_name_size += 5;
    let (current_cols, _current_rows) = termion::terminal_size().unwrap();
    let format_cols = current_cols as usize / longest_file_name_size;
    let mut printed_cols = 0;
    let mut printed_cols_newline_count = 0;
    if format_cols <= 1 {
        for file_name in &file_names {
            print!("{}\r\n", file_name.to_str().unwrap());
        }
    } else {
        for file_name in &file_names {
            print!("{}", file_name.to_str().unwrap());
            print!(
                "{}",
                " ".repeat((longest_file_name_size) - file_name.to_str().unwrap().chars().count())
            );

            printed_cols += 1;
            printed_cols_newline_count += 1;
            if ((printed_cols_newline_count + 1) > format_cols)
                || (printed_cols == file_names.len())
            {
                printed_cols_newline_count = 0;
                print!("\n\r");
            }
        }
    }
}

fn parse(input: &str) -> bool {
    let input = input.trim();
    if input == "q" || input == "exit" || input == "quit" {
        return false;
    }

    let mut split = input.splitn(2, ' ');
    let command = split.next().unwrap();
    let args = split.next().unwrap_or("").split_whitespace().map(String::from).collect::<Vec<String>>();

    launch(command, &args);
    return true;
}

fn launch(command: &str, args: &[String]) {
    if builtin(command, args) {
        return;
    }

    let mut path = PathBuf::from(command);
    if let Ok(path_var) = env::var("PATH") {
        for split_path in env::split_paths(&path_var) {
            let full_path = split_path.join(command);
            if full_path.exists() {
                path = full_path;
            }
        }
    }
    if !path.exists() {
        println!("File {} does not exist", command);
        return;
    }

    let metadata = fs::metadata(path).unwrap();
    let is_exec = metadata.permissions().mode() & 0o111 != 0;
    if !is_exec {
        println!("File {} is not executable", command);
        return;
    }

    if metadata.is_dir() {
        println!("Directory is not executable");
        return;
    }

    spawn_tty(command, args);
}

fn builtin(command: &str, args: &[String]) -> bool {
    match command {
        "cd" => {
            if args.is_empty() {
                match env::var("HOME") {
                    Ok(home) => {
                        env::set_current_dir(home).expect("Failed to change to home directory")
                    }
                    Err(_) => {
                        println!("No home directory found");
                    }
                }
            } else {
                if let Some(arg) = args.first() {
                    if let Err(error) = env::set_current_dir(Path::new(arg)) {
                        println!("cd: {}", error);
                    }
                }
            }
            return true;
        }
        _ => false,
    }
}

fn spawn_tty(command: &str, args: &[String]) {
    let _raw = std::io::stdout().into_raw_mode().unwrap();
    let stdin_fd = io::stdin().as_raw_fd();
    fcntl(stdin_fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();

    let (current_cols, current_rows) = termion::terminal_size().unwrap();
    let pty_system = portable_pty::native_pty_system();
    let pty_size: portable_pty::PtySize = portable_pty::PtySize {
        rows: current_rows,
        cols: current_cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(pty_size).unwrap();

    let mut cmd = portable_pty::CommandBuilder::new(command);
    cmd.cwd(env::current_dir().unwrap());
    for arg in args {
        cmd.arg(arg);
    }
    let child = pair.slave.spawn_command(cmd).unwrap();
    let child = Arc::new(Mutex::new(child));

    let mut reader = pair.master.try_clone_reader().unwrap();
    let read_handle = thread::spawn(move || {
        let mut buf = [0; 1024];
        while let Ok(size) = reader.read(&mut buf) {
            if size == 0 {
                break;
            }
            let mut written = 0;
            while written < size {
                match io::stdout().write(&buf[written..size]) {
                    Ok(n) => {
                        written += n;
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => panic!("Failed to write to stdout: {}", e),
                }
            }

            loop {
                match io::stdout().flush() {
                    Ok(_) => break,
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    Err(e) => panic!("Failed to flush stdout: {}", e),
                }
            }
        }
    });

    let child_clone = child.clone();
    let write_handle = thread::spawn(move || {
        let mut master_for_writing = pair.master.take_writer().unwrap();
        let mut buf = [0; 1024];
        while child_clone.lock().unwrap().try_wait().unwrap().is_none() {
            match io::stdin().read(&mut buf) {
                Ok(size) if size > 0 => {
                    if master_for_writing.write_all(&buf[..size]).is_err() {
                        break;
                    }
                    master_for_writing.flush().unwrap();
                }
                _ => thread::sleep(Duration::from_millis(10)),
            }
        }
    });

    write_handle.join().unwrap();
    read_handle.join().unwrap();
    child.lock().unwrap().wait().unwrap();

    fcntl(stdin_fd, FcntlArg::F_SETFL(OFlag::empty())).unwrap();
}
