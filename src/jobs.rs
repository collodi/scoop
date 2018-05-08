use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use std::fs::OpenOptions;

use serde_json;
use rand::{Rng, thread_rng};
use chrono::{Duration, Local};
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
        let t = parse_time(time).expect("Invalid time format.");
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
                if inserted {
                        writeln!(&mut fw, "{}", line).unwrap();
                        continue;
                }

                let line_job: Job = serde_json::from_str(&line).unwrap();
                let line_t = NaiveDateTime::parse_from_str(&line_job.time, TIME_FORMAT).unwrap();
                /* insert if new job's time is earlier than line's */
                if t < line_t {
                        serde_json::to_writer(&fw, &job).unwrap();
                        write!(&mut fw, "\n").unwrap();
                        inserted = true;
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
        println!("Job {} inserted.", job.id);
}

fn parse_time(s: &str) -> Option<NaiveDateTime> {
        let now = Local::now().naive_local();
        if s.starts_with("+") {
                let (_, t) = s.split_at(1);
                t.split('.')
                        .map(parse_duration)
                        .fold(Some(now), |acc, x| acc.and_then(|y| y.checked_add_signed(x)))
        } else {
                None
        }
}

fn parse_duration(t: &str) -> Duration {
        let (n, c) = t.split_at(t.len() - 1);
        let n = n.parse().expect("Invalid time format.");
        match c {
                "s" => Duration::seconds(n),
                "m" => Duration::minutes(n),
                "h" => Duration::hours(n),
                "d" => Duration::days(n),
                _ => panic!("Invalid time format.")
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
                let line = line.unwrap();
                let job: Job = serde_json::from_str(&line).unwrap();
                println!("{}\t{}\t{}", job.id, job.time, job.cmd);
        };
}

pub fn get_first_job() -> Option<Job> {
        let f = File::open(JOB_FILE).unwrap();

        let mut line = String::new();
        BufReader::new(f).read_line(&mut line).unwrap();
        serde_json::from_str(&line).ok()
}
