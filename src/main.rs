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

#[derive(Debug, Eq, PartialEq, Default)]
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
        if self.tracknumber > 0 {
            if self.discnumber > 0 {
                format!("disc {}, track {}", self.discnumber, self.tracknumber)
            } else {
                format!("track {}", self.tracknumber)
            }
        } else {
            String::new()
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

    if min >= 60 {
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
            notify("C* Music Player", "Unable to determine socket path", None);
            return;
        }
    };

    let mut sock = match UnixStream::connect(socket_path) {
        Ok(sock) => sock,
        Err(_) => {
            notify("C* Music Player", "Not running", None);
            return;
        }
    };

    send(&mut sock, "status\n");
    let response = recv(&mut sock);
    sock.shutdown(Shutdown::Both)
        .expect("Unable to shutdown socket");

    let m = parse(&response);

    notify(&m.get_title(), &m.get_message(), m.get_cover());
}

#[cfg(test)]
mod test_metadata {
    use super::Metadata;
    use rstest::rstest;

    #[rstest]
    #[case::no_artist_no_title("", "", "C* Music Player")]
    #[case::artist_only("L'artist", "", "C* Music Player")]
    #[case::title_only("", "Le Title", "C* Music Player")]
    #[case::title_and_artist("L'artist", "Le Title", "L'artist - Le Title")]
    fn test_get_title(#[case] artist: String, #[case] title: String, #[case] expected: String) {
        let meta = Metadata {
            artist,
            title,
            ..Default::default()
        };

        assert_eq!(meta.get_title(), expected)
    }

    #[test]
    fn test_get_message() {
        let meta = Metadata {
            album: "L'album".to_string(),
            ..Default::default()
        };

        assert_eq!(meta.get_message(), "L'album\n".to_string())
    }

    #[rstest]
    #[case::no_track_or_disc(0, 0, "")]
    #[case::disc_only(1, 0, "")]
    #[case::track_only(0, 69, "track 69")]
    #[case::track_and_disc(42, 69, "disc 42, track 69")]
    fn test_get_message_with_track(
        #[case] discnumber: u8,
        #[case] tracknumber: u8,
        #[case] expected: String,
    ) {
        let meta = Metadata {
            tracknumber,
            discnumber,
            ..Default::default()
        };

        assert_eq!(meta.get_message(), format!("\n{}", expected))
    }

    #[rstest]
    #[case::no_duration_or_position(0, 0, "")]
    #[case::position_only(1, 0, "")]
    #[case::duration_only(0, 69, ", 01:09")]
    #[case::position_and_duration(42, 69, ", 00:42 / 01:09")]
    fn test_get_message_with_duration(
        #[case] position: u32,
        #[case] duration: u32,
        #[case] expected: String,
    ) {
        let meta = Metadata {
            position,
            duration,
            ..Default::default()
        };

        assert_eq!(meta.get_message(), format!("\n{}", expected))
    }

    #[rstest]
    #[case("playing", "")]
    #[case("paused", " [Paused]")]
    #[case("stopped", " [Stopped]")]
    #[case("whatever", "")]
    #[case("", "")]
    fn test_get_message_with_status(#[case] status: String, #[case] expected: String) {
        let meta = Metadata {
            status,
            ..Default::default()
        };

        assert_eq!(meta.get_message(), format!("{}\n", expected))
    }

    #[test]
    fn test_get_message_full() {
        let meta = Metadata {
            album: "Album".to_string(),
            tracknumber: 2,
            discnumber: 1,
            position: 14,
            duration: 123,
            status: "stopped".to_string(),
            ..Default::default()
        };

        assert_eq!(
            meta.get_message(),
            String::from("Album [Stopped]\ndisc 1, track 2, 00:14 / 02:03")
        )
    }

    #[rstest]
    #[case("playing", "")]
    #[case("paused", " [Paused]")]
    #[case("stopped", " [Stopped]")]
    #[case("invalid-status", "")]
    #[case("", "")]
    fn test_get_status(#[case] status: String, #[case] expected: String) {
        let meta = Metadata {
            status,
            ..Default::default()
        };

        assert_eq!(meta.get_status(), expected);
    }

    #[rstest]
    #[case(0, 0, "")]
    #[case(3, 0, "")]
    #[case(0, 1, "track 1")]
    #[case(0, 2, "track 2")]
    #[case(1, 2, "disc 1, track 2")]
    #[case(3, 3, "disc 3, track 3")]
    fn test_get_track(#[case] discnumber: u8, #[case] tracknumber: u8, #[case] expected: String) {
        let meta = Metadata {
            tracknumber,
            discnumber,
            ..Default::default()
        };

        assert_eq!(meta.get_track(), expected);
    }

    #[rstest]
    #[case(0, 0, None)]
    #[case(0, 60, Some("01:00"))]
    #[case(58, 0, None)]
    #[case(58, 60, Some("00:58 / 01:00"))]
    fn test_get_duration(
        #[case] position: u32,
        #[case] duration: u32,
        #[case] expected: Option<&str>,
    ) {
        let meta = Metadata {
            duration,
            position,
            ..Default::default()
        };

        assert_eq!(meta.get_duration(), expected.map(|e| e.to_string()))
    }
}

#[cfg(test)]
mod test_format_time {
    use super::format_time;
    use rstest::rstest;

    #[rstest]
    #[case(0, "00:00")]
    #[case(1, "00:01")]
    #[case(59, "00:59")]
    #[case(60, "01:00")]
    #[case(61, "01:01")]
    #[case(3600, "01:00:00")]
    fn test_format_only_seconds(#[case] sec: u32, #[case] expected: String) {
        assert_eq!(format_time(sec), expected);
    }
}

#[cfg(test)]
mod test_parse {
    use super::{parse, Metadata};

    #[test]
    fn test_parse() {
        let data = "status stopped
file /music/artist/album/song.flac
duration 258
position 123
tag genre Neo Classical Fusion
tag date 1824
tag albumartist Various Artists
tag artist Metallideth
tag album Rust in Puppets
tag title Orgasmatron
tag tracknumber 69
tag discnumber 42";

        let expected = Metadata {
            file: "/music/artist/album/song.flac".to_string(),
            artist: "Metallideth".to_string(),
            album: "Rust in Puppets".to_string(),
            title: "Orgasmatron".to_string(),
            tracknumber: 69,
            discnumber: 42,
            date: "1824".to_string(),
            duration: 258,
            position: 123,
            status: "stopped".to_string(),
        };

        assert_eq!(parse(data), expected);
    }
}
