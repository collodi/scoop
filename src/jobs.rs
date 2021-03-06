use std::fs;
use std::fs::File;
use std::process::exit;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::OpenOptions;

use serde_json;
use rand::{Rng, thread_rng};
use chrono::{Duration, Local, Timelike};
use chrono::naive::NaiveDateTime;

pub const JOB_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/jobs");
const TMP_JOB_FILE: &'static str = concat!(env!("HOME"), "/.local/share/scoop/jobs.tmp");

pub const TIME_FORMAT: &'static str = "%Y-%m-%d %H:%M:%S";

#[derive(Serialize, Deserialize)]
pub struct Job {
        pub id: String,
        pub time: String,
        pub cmd: String,
        pub args: Vec<String>
}

pub fn add_job(time: &str, cmd: &str, args: &Vec<String>) {
        let t = if let Some(x) = parse_time(time) { x }
                else {
                        eprintln!("Invalid time format.");
                        exit(1);
                };

        if t < Local::now().naive_local() {
                eprintln!("Specified time is in the past.");
                exit(1);
        }

        let job = Job { 
                id: format!("{:04x}", thread_rng().gen::<u16>()),
                time: t.format(TIME_FORMAT).to_string(), 
                cmd: cmd.to_owned(),
                args: args.clone()
        };

        let mut inserted = false;
        let mut fw = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(TMP_JOB_FILE)
                .unwrap();

        let fr = File::open(JOB_FILE).unwrap();
        for line in BufReader::new(fr).lines() {
                let line = line.unwrap();
                if !inserted {
                        let ljob: Job = serde_json::from_str(&line).unwrap();
                        let ltm = NaiveDateTime::parse_from_str(&ljob.time, TIME_FORMAT).unwrap();
                        /* insert if new job's time is earlier than line's */
                        if t < ltm {
                                serde_json::to_writer(&fw, &job).unwrap();
                                write!(&mut fw, "\n").unwrap();
                                inserted = true;
                        }
                }
                /* insert the line back */
                writeln!(&mut fw, "{}", line).unwrap();
        };

        /* if not inserted, insert */
        if !inserted {
                serde_json::to_writer(&fw, &job).unwrap();
                write!(&mut fw, "\n").unwrap();
        }
        /* replace old file */
        fs::rename(TMP_JOB_FILE, JOB_FILE).unwrap();
        println!("Job {} scheduled.", job.id);
}

fn parse_time(s: &str) -> Option<NaiveDateTime> {
        let now = Local::now().naive_local();
        if s.starts_with("+") {
                let (_, t) = s.split_at(1);
                t.split('.').fold(Some(now), parse_duration)
        } else if s.starts_with("@") {
                let (_, t) = s.split_at(1);
                parse_instant(now, t)
        } else {
                None
        }
}

fn parse_instant(now: NaiveDateTime, t: &str) -> Option<NaiveDateTime> {
        let am = t.ends_with("am");
        let pm = t.ends_with("pm");
        if (!am && !pm) || t.len() > 6 || t.len() < 3 {
                return None;
        }

        let odd = t.len() % 2;
        let hh = t[0..(2 - odd)].parse().ok().filter(|&h| 0 < h && h < 13);
        let mm = if t.len() > 4 { 
                t[(2 - odd)..(4 - odd)].parse().ok().filter(|&m| m < 60) } else { 
                Some(0) };

        if hh.is_none() || mm.is_none() {
                return None;
        }

        let hh = hh.map(|h|
                if h != 12 && pm { h + 12 } 
                else if h == 12 && am { 0 } 
                else { h }
        );

        now.with_hour(hh.unwrap())
                .and_then(|n| n.with_minute(mm.unwrap()))
                .and_then(|n| n.with_second(0))
                .map(|n| if n <= now { n + Duration::days(1) } else { n })
}

fn parse_duration(now: Option<NaiveDateTime>, t: &str) -> Option<NaiveDateTime> {
        if now.is_none() {
                return None;
        }

        let now = now.unwrap();
        let (n, c) = t.split_at(t.len() - 1);
        if let Ok(n) = n.parse() {
                let a = match c {
                        "s" => Duration::seconds(n),
                        "m" => Duration::minutes(n),
                        "h" => Duration::hours(n),
                        "d" => Duration::days(n),
                        _ => return None
                };
                now.checked_add_signed(a)
        } else {
                None
        }
}

pub fn del_job(id: &str) {
        let mut fw = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(TMP_JOB_FILE)
                .unwrap();

        let fr = File::open(JOB_FILE).unwrap();
        for line in BufReader::new(fr).lines() {
                let line = line.unwrap();
                let job: Job = serde_json::from_str(&line).unwrap();

                if job.id == id {
                        println!("Job {} deleted.", id);
                } else {
                        writeln!(&mut fw, "{}", line).unwrap();
                }
        }
        fs::rename(TMP_JOB_FILE, JOB_FILE).unwrap();
}

pub fn list_jobs() {
        let f = File::open(JOB_FILE).unwrap();
        
        println!("{:4}\t{:19}\t{}", "ID", "TIME", "COMMAND");
        for line in BufReader::new(f).lines() {
                let job: Job = serde_json::from_str(&line.unwrap()).unwrap();
                println!("{}\t{}\t{} {}", job.id, job.time, job.cmd, job.args.join(" "));
        };
}

pub fn get_first_job() -> Option<Job> {
        let f = File::open(JOB_FILE).unwrap();

        let mut line = String::new();
        BufReader::new(f).read_line(&mut line).unwrap();
        serde_json::from_str(&line).ok()
}
