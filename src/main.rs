use nix::fcntl::{fcntl, FcntlArg, OFlag};
use nix::NixPath;
use std::{fs, os, path};
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
                autocomplete(&mut input);
                print!("\x1b[2K\r");
                print!("? {}", input);
                io::stdout().flush().unwrap();
            }
            Key::Char('\n') => {
                print!("\r\n");
                io::stdout().flush().unwrap();
                if !input.is_empty() && !parse(&input) {
                    //drop(_raw);
                    //std::process::exit(0);
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

fn autocomplete(input: &mut String) {
    //let last_input = input.trim().split(" ").last().unwrap();

    let mut file_names: Vec<PathBuf> = fs::read_dir(".")
        .unwrap()
        .filter_map(|dir| dir.ok())
        .map(|entry| {
            entry.path().strip_prefix("./").unwrap().to_path_buf()
        })
        .filter(|file_name| {
            file_name.to_str().unwrap().starts_with(input.trim())
        })
        .collect();

    if file_names.is_empty() {
        return;
    }

    file_names.sort_by_key(|path| {
        return path.len();
    });
/* 
    println!("\r");
    for f in file_names.to_owned() {
        println!("{}", f.to_str().unwrap());
        print!("\r");
    }
*/
    let first = &file_names[0]
        .to_str().unwrap();//[1..];
    let mut prefix_len = first.len();

    for s in file_names.iter().skip(1) {
        prefix_len = first
            .chars()
            .zip(s.to_str().unwrap().chars())
            .take_while(|(a, b)| a == b)
            .count()
            .min(prefix_len)
    }
    // println!("common: {}\r", first.to_str().unwrap()[..prefix_len].to_string());
     
    *input = first[..prefix_len].to_string();
    if prefix_len == first.len() {
        list_files(file_names);
        //print!("? {}", &input);
        //io::stdout().flush().unwrap();
    }

    print!("{}", input);
    io::stdout().flush().unwrap();
    //list_files(file_names);
}

fn list_files(file_names: Vec<PathBuf>) {
    print!("\r\n");
    io::stdout().flush().unwrap();
    // Get size in characters of longest file name
    let mut longest_file_name_size = file_names.iter()
        .map(|name| name.to_str().unwrap().chars().count()).max().unwrap_or(0);

    // Add padding
    longest_file_name_size += 5;

    // Get terminal width
    let (current_cols, current_rows) = termion::terminal_size().unwrap();
    //println!("Cols: {}, Rows: {}, Longest entry: {}\r", current_cols, current_rows, longest_file_name_size);

    // Get max cols of entries
    let mut format_cols = current_cols as usize / longest_file_name_size;
    //println!("Format cols: {}, {}", format_cols, file_names.len());
    //if format_cols > file_names.len() {
    //    format_cols = file_names.len();
    //}

    // Get width of formatted output
    let _w = (longest_file_name_size + 1) * 2;
     
    //print!("\n\r");
    let mut printed_cols = 0;
    let mut printed_cols_newline_count = 0;
    if format_cols <= 1 {
        for file_name in &file_names {
            print!("{}\r\n", file_name.to_str().unwrap());
        }
    } else {
        for file_name in &file_names {
            print!("{}", file_name.to_str().unwrap());
            print!("{}", " ".repeat((longest_file_name_size) - file_name.to_str().unwrap().chars().count()));

            printed_cols += 1;
            printed_cols_newline_count += 1; 
            if ((printed_cols_newline_count + 1) > format_cols) || (printed_cols == file_names.len()) {
                printed_cols_newline_count = 0;
                print!("\n\r");
            }
        }
    }
    //print!("\r\n{}", input);
    //io::stdout().flush().unwrap();

    
}

fn parse(input: &str) -> bool {
    let input = input.trim();
    if input == "q" || input == "exit" || input == "quit" {
        return false;
    }

    let mut split = input.splitn(2, ' ');
    let command = split.next().unwrap();
    let args = split.next().unwrap_or("");

    // println!("Command: '{}', Args: '{}'", command, args);

    launch(command, args);
    return true;
}

fn launch(command: &str, args: &str) {
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

    /*if !path.is_file() {
        println!("'{}' is not a file", command);
        return;
    }*/

    spawn_tty(command, args);
    return;
}

fn builtin(command: &str, args: &str) -> bool {
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
                if let Err(error) = env::set_current_dir(Path::new(args)) {
                    println!("cd: {}", error);
                }
            }
            return true;
        }
        _ => false,
    }
}

fn spawn_tty(command: &str, args: &str) {
    // Set raw mode
    let _raw = std::io::stdout().into_raw_mode().unwrap();
    // Set non-blocking stdin
    let stdin_fd = io::stdin().as_raw_fd();
    fcntl(stdin_fd, FcntlArg::F_SETFL(OFlag::O_NONBLOCK)).unwrap();

    // Pty setup
    let (current_cols, current_rows) = termion::terminal_size().unwrap();
    let pty_system = portable_pty::native_pty_system();
    let pty_size: portable_pty::PtySize = portable_pty::PtySize {
        rows: current_rows,
        cols: current_cols,
        pixel_width: 0,
        pixel_height: 0,
    };

    let pair = pty_system.openpty(pty_size).unwrap();

    // Spawn command
    let mut cmd = portable_pty::CommandBuilder::new(command);
    cmd.cwd(env::current_dir().unwrap());
    if !args.is_empty() {
        cmd.arg(args);
    }
    let child = pair.slave.spawn_command(cmd).unwrap();
    let child = Arc::new(Mutex::new(child));

    // Pty reader thread
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

    // Clear non-blocking stdin
    fcntl(stdin_fd, FcntlArg::F_SETFL(OFlag::empty())).unwrap();
}
