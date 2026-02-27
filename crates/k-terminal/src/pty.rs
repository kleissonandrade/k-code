use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;

use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};

pub struct PtyHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    output_rx: mpsc::Receiver<Vec<u8>>,
    _reader_thread: thread::JoinHandle<()>,
}

impl PtyHandle {
    pub fn spawn(cols: u16, rows: u16, cwd: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut cmd = CommandBuilder::new(&shell);
        cmd.cwd(cwd);
        // Set TERM so programs know we support colors
        cmd.env("TERM", "xterm-256color");

        pair.slave.spawn_command(cmd)?;
        // Drop the slave side — we only need the master
        drop(pair.slave);

        let writer = pair.master.take_writer()?;

        let (tx, rx) = mpsc::channel();
        let mut reader = pair.master.try_clone_reader()?;

        let reader_thread = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            master: pair.master,
            writer,
            output_rx: rx,
            _reader_thread: reader_thread,
        })
    }

    pub fn write_input(&mut self, data: &[u8]) {
        let _ = self.writer.write_all(data);
        let _ = self.writer.flush();
    }

    pub fn try_read_output(&self) -> Option<Vec<u8>> {
        let mut combined = Vec::new();
        while let Ok(chunk) = self.output_rx.try_recv() {
            combined.extend(chunk);
        }
        if combined.is_empty() {
            None
        } else {
            Some(combined)
        }
    }

    pub fn resize(&self, cols: u16, rows: u16) {
        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
