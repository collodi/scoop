#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate daemonize;
extern crate chrono;
extern crate docopt;
extern crate rand;

mod jobs;
use jobs::*;

use std::fs;
use std::fs::File;
use std::path::Path;
use std::error::Error;
use std::fs::OpenOptions;

use daemonize::Daemonize;
use chrono::Local;
use docopt::Docopt;
use serde_json::Value;

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
  scoop add <time> <command>
  scoop del <id>
  scoop (-h | --help)
  scoop --version

Options:
  -h --help     Show this message.
  --version     Show version.
";

#[derive(Deserialize)]
struct Args {
        flag_version: bool,
        arg_time: String,
        arg_command: String,
        arg_id: String,
        cmd_daemon: bool,
        cmd_list: bool,
        cmd_add: bool,
        cmd_del: bool
}

fn main() {
        let args: Args = Docopt::new(USAGE)
                .and_then(|d| d.deserialize())
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
                add_job(&args.arg_time, &args.arg_command);
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
                        /* TODO wait */
                },
                Err(e) => {
                        logerr("daemon failed to start.", e);
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
