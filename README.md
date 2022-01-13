# Music Playlist Downloader

Project status : PoC

## Features

- [x] Get a list of songs from a Youtube playlist
- [x] CLI
- [x] Output the list in a text file
- [x] Download the songs with a web API
- [x] Download progress bar
- [ ] Write a documentation "How to use djownload"
- [ ] Make different crates
- [ ] Properly handle errors, results
- [ ] CI/CD on new releases : build and push docker image
- [ ] Log messages for debugging [log crate](https://crates.io/crates/log)
- [ ] Downloaded songs are placed in a folder with the same name as the playlist
- [ ] CLI takes a playlist URL as an argument and not a playlist ID
- [ ] Refactor the code to handle different download websites
- [ ] Fallback to other download websites if the first one fails

## Usage

Download a playlist from Youtube playlist url :

`music-playlist-downloader https://www.youtube.com/playlist?list=PL4cUxeGkcC9gQ-qjXZQjzXuJXQXQZjL9v`

Output a Youtube playlist in a text file :

`music-playlist-downloader https://www.youtube.com/playlist?list=PL4cUxeGkcC9gQ-qjXZQjzXuJXQXQZjL9v --output-file=playlist.txt`
