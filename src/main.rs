#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate daemonize;
extern crate chrono;
extern crate docopt;
extern crate rand;
extern crate notify;
extern crate libc;

mod jobs;
use jobs::*;

use std::fs;
use std::thread;
use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::time::Duration;
use std::fs::OpenOptions;
use std::process::Command;
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex, Condvar};
use std::os::unix::process::CommandExt;

use libc::{wait, fork, setsid};
use daemonize::Daemonize;
use chrono::{Local, NaiveDateTime};
use docopt::Docopt;
use notify::{RecommendedWatcher, Watcher, RecursiveMode, RawEvent};

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const DATA_DIR: &'static str = concat!(env!("HOME"), "/.local/share/scoop/");
const PID_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/scoop.pid");
const LOG_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/logs");
const ERR_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/errors");

const USAGE: &'static str = "
Dumb scheduler here.

Usage:
    scoop daemon
    scoop list
    scoop add <time> <command> [<args>...]
    scoop del <id>
    scoop (-h | --help)
    scoop --version

Options:
    -h --help     Show this message.
    --version     Show version.

You can specify time in one of two ways:
    +<offset>
    @<instant>

The grammar for time is specified here:
    <offset> := <N>(d|h|m|s)[.<offset>]
    <instant> := <N>(Y|M|D|h|m|s)[.<instant>]
";

#[derive(Deserialize, Debug)]
struct Args {
        flag_version: bool,
        arg_time: String,
        arg_command: String,
        arg_args: Vec<String>,
        arg_id: String,
        cmd_daemon: bool,
        cmd_list: bool,
        cmd_add: bool,
        cmd_del: bool
}

fn main() {
        let args: Args = Docopt::new(USAGE)
                .and_then(|d| d.options_first(true).deserialize())
                .unwrap_or_else(|e| e.exit());

        if !Path::new(DATA_DIR).exists() {
                fs::create_dir_all(DATA_DIR).unwrap();
        }

        if !Path::new(JOB_FILE).exists() {
                File::create(JOB_FILE).unwrap();
        }
        
        /* show version */
        if args.flag_version {
                println!("scoop v{}", VERSION);
                std::process::exit(0);
        }

        if args.cmd_list {
                list_jobs();
        } else if args.cmd_add {
                add_job(&args.arg_time, &args.arg_command, &args.arg_args);
        } else if args.cmd_daemon {
                start_daemon();
        } else if args.cmd_del {
                del_job(&args.arg_id);
        }
}

fn start_daemon() {
        let stdout = OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(LOG_FILE)
                .unwrap();

        let stderr = OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(ERR_FILE)
                .unwrap();

        let daemon = Daemonize::new()
                .pid_file(PID_FILE)
                .stdout(stdout)
                .stderr(stderr);

        match daemon.start() {
                Ok(_) => {
                        log("daemon started.");
                        spawn_daemon();
                },
                Err(e) => {
                        println!("daemon failed to start.");
                        println!("{}", e);
                }
        }
}

fn log(msg: &str) {
        println!("{}: {}", Local::now().format(TIME_FORMAT), msg);
}

fn logerr<T: Error>(msg: &str, err: T) {
        eprintln!("{}: {}", Local::now().format(TIME_FORMAT), msg);
        eprintln!("{}: {}", Local::now().format(TIME_FORMAT), err);
}

fn spawn_daemon() {
        let pair = Arc::new((Mutex::new(false), Condvar::new()));
        let pair2 = pair.clone();

        thread::spawn(move || {
                let &(ref lock, ref cvar) = &*pair2;

                loop {
                        let res = watch_jobs();
                        let mut stop = lock.lock().unwrap();
                        if let Err(e) = res {
                                logerr("fs watcher died.", e);

                                *stop = true;
                                cvar.notify_one();
                                break;
                        }

                        cvar.notify_one();
                }
        });

        let &(ref lock, ref cvar) = &*pair;
        let mut stop = lock.lock().unwrap();
        loop {
                let mut dur = Duration::from_secs(3600);
                if let Some(job) = get_first_job() {
                        let t = NaiveDateTime::parse_from_str(&job.time, TIME_FORMAT).unwrap();
                        let now = Local::now().naive_local();

                        let sub = t.signed_duration_since(now).num_seconds();
                        if sub < 0 {
                                del_job(&job.id);
                        } else if sub == 0 { 
                                del_job(&job.id);
                                /* if current, do it */
                                exec_job(&job);
                        } else { 
                                /* if future, calc wait time */
                                dur = Duration::from_secs(sub as u64);
                        }
                }
                
                let res = cvar.wait_timeout(stop, dur).unwrap();
                stop = res.0;
                if *stop {
                        break;
                }
        }
}

fn exec_job(job: &Job) {
        unsafe {
                let mut fk = fork();
                let ptr: *mut i32 = &mut fk;

                if fk == 0 {
                        setsid();
                        
                        let rc = fork();
                        if rc == 0 {
                                let res = Command::new(&job.cmd).args(&job.args).exec();
                                logerr(&format!("command '{}' failed to start.", job.cmd), res);
                        } else {
                                std::process::exit(0);
                        }
                } else {
                        wait(ptr);
                }
        }
}

fn watch_jobs() -> notify::Result<RawEvent> {
        let (tx, rx) = channel();
        let mut watcher: RecommendedWatcher = try!(Watcher::new_raw(tx));
        try!(watcher.watch(JOB_FILE, RecursiveMode::Recursive));

        rx.recv().map_err(|e| notify::Error::Generic(format!("{}", e)))
}
