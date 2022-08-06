# cmus-notify

Simple Rust app to show ![cmus](https://github.com/cmus/cmus) notifications.

### Compiling
Build with cargo:
```
cargo build
```

### Usage
Set cmus-notify as your notification program with the following config:
```
set status_display_program=/path/to/cmus-notify
```

### Covers
cmus-notify will use check for a file named cover.jpg or cover.png in the same folder as the
currently playing file.

### Example
![Example](https://github.com/mathieu-lemay/cmus-notify/blob/master/example.png)
