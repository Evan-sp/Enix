use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::os::unix::io::AsRawFd;
use std::{
    env,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use termion::raw::IntoRawMode;

fn main() {
    let mut input = String::new();
    loop {
        input.clear();

        print!("? ");
        io::stdout().flush().unwrap();
        io::stdin().read_line(&mut input).unwrap();

        if !parse(&input) {
            break;
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
    let args = split.next().unwrap_or("");

    println!("Command: '{}', Args: '{}'", command, args);

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
    if !path.is_file() {
        println!("'{}' is not a file", command);
        return;
    }

    spawn_tty(command, args);
    return;
}

fn builtin(command: &str, args: &str) -> bool {
    match command {
        "cd" => {
            if args.is_empty() {
                match env::var("HOME") {
                    Ok(home) => env::set_current_dir(home).expect("Failed to change to home directory"),
                    Err(_) => {
                        println!("No home directory found");
                    }
                }
            } else {
                env::set_current_dir(Path::new(args)).unwrap();
            }
            return true;
        }
        _ => false,
    }
}

fn spawn_tty(command: &str, _args: &str) {
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
    //cmd.arg(args);
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
