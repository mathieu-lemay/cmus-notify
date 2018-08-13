#! /usr/bin/env zsh -eu

cargo build

time cmus-notify status playing file /Users/mathieu/Music/chthe_ilist_-_le_dernier_crepuscule/05_-_the_voices_from_beneath_the_well.flac artist "Chthe'ilist" album "Le Dernier Crépuscule" tracknumber 05 title "The Voices from Beneath the Well" date 2016 duration 460 position 340
time ./target/debug/cmus-notify-rust status playing file /Users/mathieu/Music/chthe_ilist_-_le_dernier_crepuscule/05_-_the_voices_from_beneath_the_well.flac artist "Chthe'ilist" album "Le Dernier Crépuscule" tracknumber 05 title "The Voices from Beneath the Well" date 2016 duration 460 position 340
