#[cfg(target_os = "linux")]
use std::env;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

#[derive(Default)]
struct Metadata {
    file: String,
    artist: String,
    album: String,
    title: String,
    tracknumber: u8,
    discnumber: u8,
    date: String,
    duration: u32,
    position: u32,
    status: String,
}

impl Metadata {
    fn get_title(&self) -> String {
        if self.artist != "" && self.album != "" {
            return format!("{} - {}", self.artist, self.title);
        } else {
            return String::from("C* Music Player");
        }
    }

    fn get_message(&self) -> String {
        let mut body = format!("{}{}\n{}", self.album, self.get_status(), self.get_track());

        let duration = self.get_duration();

        match duration {
            Some(s) => body.push_str(&format!(", {}", s)),
            None => (),
        };

        return body;
    }

    fn get_cover(&self) -> Option<PathBuf> {
        if self.file == "" {
            return None;
        }

        let file_path = Path::new(&self.file);
        let directory = file_path.parent();

        if directory.is_none() {
            return None;
        }

        let directory = directory.unwrap();

        let mut cover = PathBuf::from(directory);
        cover.push("cover.jpg");

        if cover.exists() {
            return Some(cover);
        }

        let mut cover = PathBuf::from(directory);
        cover.push("cover.png");

        if cover.exists() {
            return Some(cover);
        }

        return None;
    }

    fn get_status(&self) -> String {
        match self.status.as_str() {
            "playing" => String::from(""),
            "paused" => String::from(" [Paused]"),
            "stopped" => String::from(" [Stopped]"),
            _ => String::from(""),
        }
    }

    fn get_track(&self) -> String {
        if self.discnumber > 0 {
            return format!("disc {}, track {}", self.discnumber, self.tracknumber);
        } else {
            return format!("track {}", self.tracknumber);
        }
    }

    fn get_duration(&self) -> Option<String> {
        if self.duration == 0 {
            return None;
        }

        if self.position > 0 {
            return Some(format!(
                "{} / {}",
                format_time(self.position),
                format_time(self.duration)
            ));
        } else {
            return Some(format_time(self.duration));
        }
    }
}

fn send(sock: &mut UnixStream, msg: &String) {
    let bc = match sock.write(msg.as_bytes()) {
        Ok(bc) => bc,
        Err(e) => panic!("Error writing to socket: {:?}", e),
    };

    if bc != msg.len() {
        panic!("Error writing to socket",);
    }
}

fn recv(sock: &mut UnixStream) -> String {
    const BUFSIZE: usize = 2048;
    let mut buf: [u8; BUFSIZE] = [0; BUFSIZE];
    let mut resp = String::new();

    loop {
        let bc = match sock.read(&mut buf) {
            Ok(v) => v,
            Err(e) => panic!("Error reading from socket: {:?}", e),
        };

        let chunk = String::from_utf8(buf[..bc].to_vec()).unwrap();
        resp.push_str(chunk.as_str());

        if chunk.ends_with("\n\n") {
            break;
        }
    }

    return resp;
}

fn parse(data: &String) -> Metadata {
    let mut m: Metadata = Metadata::default();

    for line in data.lines() {
        let line: Vec<&str> = line.splitn(2, ' ').collect();
        match line[0] {
            "status" => m.status = String::from(line[1]),
            "file" => m.file = String::from(line[1]),
            "duration" => m.duration = line[1].parse().unwrap(),
            "position" => m.position = line[1].parse().unwrap(),
            "tag" => {
                let tag: Vec<&str> = line[1].splitn(2, ' ').collect();

                match tag[0] {
                    "title" => m.title = String::from(tag[1]),
                    "artist" => m.artist = String::from(tag[1]),
                    "album" => m.album = String::from(tag[1]),
                    "date" => m.date = String::from(tag[1]),
                    "tracknumber" => m.tracknumber = tag[1].parse().unwrap(),
                    "discnumber" => m.discnumber = tag[1].parse().unwrap(),
                    _ => {}
                };
            }
            _ => {}
        }
    }

    return m;
}

#[cfg(target_os = "linux")]
fn notify(title: &String, msg: &String, cover: Option<PathBuf>) {
    let program = "notify-send";
    let mut args = vec!["--hint=int:transient:1"];

    args.push("--icon");
    if cover.is_some() {
        cover
            .as_ref()
            .and_then(|c| c.to_str())
            .map(|c| args.push(c));
    } else {
        args.push("applications-multimedia");
    }

    args.push(title);
    args.push(msg);

    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to execute process");
}

#[cfg(target_os = "macos")]
fn notify(title: &String, msg: &String, cover: Option<PathBuf>) {
    let program = "terminal-notifier";
    let mut args = vec!["-group", "cmus", "-title", title, "-message", msg];

    cover.as_ref().and_then(|c| c.to_str()).map(|c| {
        args.push("-appIcon");
        args.push(c)
    });

    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to execute process");
}

#[cfg(target_os = "linux")]
fn get_socket_path() -> PathBuf {
    let mut socket_path: PathBuf;

    match env::var("XDG_RUNTIME_DIR") {
        Ok(val) => socket_path = PathBuf::from(val),
        Err(_) => panic!("XDG_RUNTIME_DIR not set"),
    }
    socket_path.push("cmus-socket");

    return socket_path;
}

#[cfg(target_os = "macos")]
fn get_socket_path() -> PathBuf {
    let mut socket_path = dirs::home_dir().expect("Unable to get home dir");
    socket_path.push(".config");
    socket_path.push("cmus");
    socket_path.push("socket");

    return socket_path;
}

fn format_time(sec: u32) -> String {
    let mut sec = sec;
    let mut min: u32;
    let mut hour: u32 = 0;

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

fn main() {
    let mut sock = match UnixStream::connect(get_socket_path()) {
        Ok(sock) => sock,
        Err(_) => {
            notify(
                &String::from("C* Music Player"),
                &String::from("Not running"),
                None,
            );
            return;
        }
    };

    send(&mut sock, &String::from("status\n"));
    let response = recv(&mut sock);
    sock.shutdown(Shutdown::Both)
        .expect("Unable to shutdown socket");

    let m = parse(&response);

    notify(&m.get_title(), &m.get_message(), m.get_cover());
}
