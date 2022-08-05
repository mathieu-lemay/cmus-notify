use std::fmt::Write as _;
use std::io::Read;
use std::io::Write;
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::path::PathBuf;

use notify_rust::Notification;

#[cfg(target_os = "linux")]
use notify_rust::Hint;

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
        if !self.artist.is_empty() && !self.title.is_empty() {
            format!("{} - {}", self.artist, self.title)
        } else {
            String::from("C* Music Player")
        }
    }

    fn get_message(&self) -> String {
        let mut body = format!("{}{}\n{}", self.album, self.get_status(), self.get_track());

        let duration = self.get_duration();

        if let Some(s) = duration {
            // TODO: Handle properly.
            write!(body, ", {}", s).expect("Unable to add duration to message");
        };

        body
    }

    fn get_cover(&self) -> Option<PathBuf> {
        if self.file.is_empty() {
            return None;
        }

        let file_path = Path::new(&self.file);
        let directory = file_path.parent()?;

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

        None
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
            format!("disc {}, track {}", self.discnumber, self.tracknumber)
        } else {
            format!("track {}", self.tracknumber)
        }
    }

    fn get_duration(&self) -> Option<String> {
        if self.duration == 0 {
            return None;
        }

        if self.position > 0 {
            Some(format!(
                "{} / {}",
                format_time(self.position),
                format_time(self.duration)
            ))
        } else {
            Some(format_time(self.duration))
        }
    }
}

fn send(sock: &mut UnixStream, msg: &str) {
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

    resp
}

fn parse(data: &str) -> Metadata {
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

    m
}

fn notify(title: &str, msg: &str, cover: Option<PathBuf>) {
    let icon = if cover.is_some() {
        if let Some(c) = cover.as_ref().and_then(|c| c.to_str()) {
            c
        } else {
            "applications-multimedia"
        }
    } else {
        "applications-multimedia"
    };

    send_notification(title, msg, icon);
}

#[cfg(target_os = "linux")]
fn send_notification(title: &str, msg: &str, icon: &str) {
    Notification::new()
        .summary(title)
        .body(msg)
        .icon(icon)
        .hint(Hint::Transient(true))
        .show()
        .expect("Error showing notification.");
}

#[cfg(target_os = "macos")]
fn send_notification(title: &str, msg: &str, icon: &str) {
    Notification::new()
        .summary(title)
        .body(msg)
        .icon(icon)
        .show()
        .expect("Error showing notification.");
}

fn get_socket_path() -> Option<PathBuf> {
    if let Some(mut path) = dirs::runtime_dir() {
        path.push("cmus-socket");

        return Some(path);
    }

    if let Some(mut path) = dirs::home_dir() {
        path.push(".config");
        path.push("cmus");
        path.push("socket");

        return Some(path);
    }

    None
}

fn format_time(sec: u32) -> String {
    let mut sec = sec;
    let mut min = sec / 60;
    let mut hour: u32 = 0;

    sec %= 60;

    if min > 60 {
        hour = min / 60;
        min %= 60;
    }

    if hour != 0 {
        format!("{:02}:{:02}:{:02}", hour, min, sec)
    } else {
        format!("{:02}:{:02}", min, sec)
    }
}

fn main() {
    let socket_path = match get_socket_path() {
        Some(p) => p,
        None => {
            notify(
                &String::from("C* Music Player"),
                &String::from("Unable to determine socket path"),
                None,
            );
            return;
        }
    };

    let mut sock = match UnixStream::connect(socket_path) {
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

#[cfg(test)]
mod test_metadata {
    use super::Metadata;

    #[test]
    fn test_get_title() {
        let meta = Metadata {
            artist: String::from("L'Artist"),
            title: String::from("Le Title"),
            ..Default::default()
        };

        assert_eq!(meta.get_title(), String::from("L'Artist - Le Title"))
    }

    #[test]
    fn test_get_title_fallback_when_info_is_missing() {
        for (artist, title) in vec![
            (String::new(), String::new()),
            (String::from("L'artist"), String::new()),
            (String::new(), String::from("Le Title")),
        ] {
            let meta = Metadata {
                artist,
                title,
                ..Default::default()
            };

            assert_eq!(meta.get_title(), String::from("C* Music Player"))
        }
    }

    #[test]
    fn test_get_message_without_duration() {
        let meta = Metadata {
            album: String::from("L'Album"),
            status: String::from("paused"),
            tracknumber: 1,
            ..Default::default()
        };

        assert_eq!(
            meta.get_message(),
            String::from("L'Album [Paused]\ntrack 1")
        )
    }

    #[test]
    fn test_get_message_with_duration() {
        let meta = Metadata {
            album: String::from("L'Album"),
            status: String::from("playing"),
            tracknumber: 2,
            duration: 123,
            ..Default::default()
        };

        assert_eq!(meta.get_message(), String::from("L'Album\ntrack 2, 02:03"))
    }
}
