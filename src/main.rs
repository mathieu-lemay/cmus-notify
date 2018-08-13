use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

fn parse_args() -> HashMap<String, String> {
    let mut args: Vec<String> = env::args().collect();
    let mut map: HashMap<String, String> = HashMap::new();

    args.remove(0);
    for i in 0..args.len() / 2 {
        map.insert(args[i*2].to_owned(), args[i*2+1].to_owned());
    }

    return map;
}

fn format_track(args: &HashMap<String, String>) -> String {
    let discnumber = args.get("discnumber");
    let tracknumber = args.get("tracknumber");

    if discnumber.is_some() {
        return format!("disc {}, track {}", discnumber.unwrap(), tracknumber.unwrap());
    } else {
        return format!("track {}", tracknumber.unwrap());
    }
}

fn format_time(nb_sec_str: &String) -> String {
    let mut sec: i32 = nb_sec_str.parse().unwrap();
    let mut min: i32;
    let mut hour: i32 = 0;

    min = sec / 60;
    sec = sec % 60;

    if min > 60 {
        hour = min / 60;
        min = min % 60;
    }

    if hour != 0 {
        return format!("{:02}:{:02}:{:02}", hour, min, sec);
    } else {
        return format!("{:02}:{:02}", min, sec);
    }
}

fn format_position(args: &HashMap<String, String>) -> Option<String> {
    let duration = args.get("duration");
    let position = args.get("position");

    if duration.is_none() {
        return None;
    }

    if position.is_some() {
        return Some(format!("{} / {}", format_time(position.unwrap()), format_time(duration.unwrap())));
    } else {
        return Some(format_time(duration.unwrap()));
    }
}

fn get_cover(args: &HashMap<String, String>) -> Option<PathBuf> {
    let file_path = Path::new(&args["file"]);
    let directory = file_path.parent();

    if directory.is_none() {
        return None;
    }

    for entry in fs::read_dir(directory.unwrap()).unwrap() {
        let entry = entry.unwrap().path();
        if entry.is_file() {
            let file_name = entry.file_name().unwrap();
            if file_name == "cover.jpg" || file_name == "cover.png" {
                return Some(entry.to_owned());
            }
        }
    }

    return None;
}

fn main() {
    let args = parse_args();

    let title: String;
    let mut body: String;

    if args["status"] == "playing" {
        title = format!("{} - {}", args["artist"], args["title"]);
        body = format!("{}\n{}", args["album"], format_track(&args));

        let position = format_position(&args);

        if position.is_some() {
            body.push_str(&format!(", {}", position.unwrap()));
        }
    } else if args["status"] == "paused" {
        title = String::from("C* Music Player");
        body = String::from("Paused");
    } else if args["status"] == "stopped" {
        title = String::from("C* Music Player");
        body = String::from("Stopped");
    } else {
        title = String::from("C* Music Player");
        body = args["status"].to_owned();
    }

    let cover = get_cover(&args);

    let program = "terminal-notifier";
    let mut args = vec!["-group", "cmus", "-title", &title, "-message", &body];

    cover.as_ref()
        .and_then(|c| c.to_str())
        .map(|c| {
            args.push("-appIcon");
            args.push(c)
        });

    println!("{:?}", args);

    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to execute process");
}

