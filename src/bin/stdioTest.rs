use std::{
    io::{self, Read, Write},
    thread, time::Duration,
};
use termion::raw::IntoRawMode;
use std::os::unix::io::AsRawFd;
use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::sync::Arc;
use std::sync::Mutex;

fn main() {
    let mut input = String::new();
    while input.trim() != "q" {
        input.clear();

        print!("input: ");
        io::stdout().flush().unwrap();
        io::stdin()
            .read_line(&mut input)
            .expect("Failed to read line");

        //println!("You entered: {}", input.trim());
        parse(&input);
    }
}

fn parse(input: &str) {
    launch(input.trim());
}

fn launch(_command: &str) {
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
    let cmd = portable_pty::CommandBuilder::new("top");
    //cmd.arg("name");
    let child = pair.slave.spawn_command(cmd).unwrap();
    //let child_pid = i32::try_from(child.process_id().unwrap()).unwrap();
    let child = Arc::new(Mutex::new(child));
    let child_clone = child.clone();

    // Raw mode
    let _raw = std::io::stdout().into_raw_mode().unwrap();

    let mut reader = pair
        .master
        .try_clone_reader()
        .expect("Failed to clone PTY reader");
    let read_handle = thread::spawn(move || {
        let mut buf = [0; 1024];
        while let Ok(size) = reader.read(&mut buf) {
            if size == 0 {
                break;
            }
            io::stdout().write_all(&buf[..size]).unwrap();
            io::stdout().flush().unwrap();
        }
    });

    let write_handle = thread::spawn(move || {
        //let mut child_borrow = child_rx.recv().unwrap();
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
    //println!("start");
    write_handle.join().unwrap();
    read_handle.join().unwrap();
    child.lock().unwrap().wait().unwrap();
    //println!("end");
    fcntl(stdin_fd, FcntlArg::F_SETFL(OFlag::empty())).unwrap();
}
