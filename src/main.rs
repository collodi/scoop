#[macro_use]
extern crate serde_derive;
extern crate daemonize;
extern crate chrono;
extern crate docopt;

use std::fs;
use std::path::Path;
use std::error::Error;
use std::fs::OpenOptions;
use daemonize::Daemonize;
use chrono::Local;
use docopt::Docopt;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

const DATA_DIR: &'static str = concat!(env!("HOME"), "/.local/share/scoop/");
const PID_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/scoop.pid");
const JOB_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/jobs");
const LOG_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/logs");
const ERR_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/errors");

const TIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

const USAGE: &'static str = "
Dumb scheduler here.

Usage:
  scoop daemon
  scoop list
  scoop add <time> <command>
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
        cmd_daemon: bool,
        cmd_list: bool,
        cmd_add: bool
}

fn main() {
        let args: Args = Docopt::new(USAGE)
                .and_then(|d| d.deserialize())
                .unwrap_or_else(|e| e.exit());

        if !Path::new(DATA_DIR).exists() {
                fs::create_dir_all(DATA_DIR).unwrap();
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
        }
}

fn list_jobs() {
        if !Path::new(JOB_FILE).exists() {
                println!("No jobs.");
                return;
        }

        let f = File::open(JOB_FILE).unwrap();
        BufReader::new(f).lines().for_each(|j| {
                /* TODO print a job */
        });
}

fn add_job(time: &str, cmd: &str) {

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
                        logerr("daemon failed to start.");
                        logerr(e);
                }
        }
}

fn log(msg: &str) {
        println!("{}: {}", Local::now().format(TIME_FORMAT), msg);
}

fn logerr<T: Error>(msg: T) {
        eprintln!("{}: {}", Local::now().format(TIME_FORMAT), msg);
}
