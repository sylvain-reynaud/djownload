use serde_json::Value;
use std::env;
use std::fs::File;
use std::io::Write;

use async_recursion::async_recursion;
use clap::Parser;
use google_youtube3::YouTube;
use serde::{Deserialize, Serialize};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

use std::cmp::min;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;

pub async fn download_file(client: &Client, url: &str, path: &str) -> Result<(), String> {
    let res = client
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            "User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
        )
        .send()
        .await
        .or(Err(format!("Failed to GET from '{}'", &url)))?;
    let total_size = res
        .content_length()
        .ok_or(format!("Failed to get content length from '{}'", &url))?;

    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .progress_chars("█  "));
    pb.set_message(format!("Downloading {}", &url));

    let mut file = File::create(path).or(Err(format!("Failed to create file '{}'", path)))?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item.or(Err(format!("Error while downloading file")))?;
        file.write(&chunk)
            .or(Err(format!("Error while writing to file")))?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {}", path));
    return Ok(());
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SliderResult {
    pub audios: Audios,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Audios {
    #[serde(rename = "")]
    pub field: Vec<GeneratedType>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedType {
    pub id: String,
    pub duration: i64,
    #[serde(rename = "tit_art")]
    pub tit_art: String,
    pub url: String,
    pub extra: Value,
}

#[derive(Parser)]
#[clap(version = "0.1", author = "@sylvainreynaud")]
struct CLIOpts {
    /// Youtube Playlist ID
    playlist_id: String,

    /// output file
    #[clap(short, long)]
    output_file: Option<String>,
}

// Get the list of songs from a youtube playlist url
async fn get_song_names_from_playlist_url(
    url: String,
) -> Result<Vec<String>, google_youtube3::Error> {
    // Read application secret from a file. Sometimes it's easier to compile it directly into
    // the binary. The clientsecret file contains JSON like `{"installed":{"client_id": ... }}`
    let secret = yup_oauth2::read_application_secret("clientsecret.json")
        .await
        .expect("clientsecret.json");

    // Create an authenticator that uses an InstalledFlow to authenticate. The
    // authentication tokens are persisted to a file named tokencache.json. The
    // authenticator takes care of caching tokens to disk and refreshing tokens once
    // they've expired.
    let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("tokencache.json")
        .build()
        .await
        .unwrap();

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        auth,
    );

    // Get the list of songs from the playlist
    let result = hub
        .playlist_items()
        .list(&vec!["snippet".into()])
        .playlist_id(&url)
        .max_results(50)
        .doit()
        .await;

    match result {
        #[warn(unused_variables)]
        Ok((_response, playlist_item_list_response)) => {
            let mut song_names = Vec::new();
            extract_songs(&playlist_item_list_response, &mut song_names);

            if let Some(value) = get_all_songs_from_pagination_recursive(
                playlist_item_list_response,
                &mut song_names,
                hub,
                url,
            )
            .await
            {
                return Ok(value?);
            }
            Ok(song_names)
        }
        Err(e) => Err(google_youtube3::Error::from(e)),
    }
}

#[async_recursion]
async fn get_all_songs_from_pagination_recursive(
    playlist_item_list_response: google_youtube3::api::PlaylistItemListResponse,
    song_names: &mut Vec<String>,
    hub: YouTube,
    url: String,
) -> Option<google_youtube3::Result<Vec<String>>> {
    // extract_songs(&playlist_item_list_response, song_names);

    while playlist_item_list_response.next_page_token.is_some() {
        let next_page_token = playlist_item_list_response
            .next_page_token
            .as_ref()
            .unwrap();

        println!("Next page token: {}", next_page_token);

        let result = hub
            .playlist_items()
            .list(&vec!["snippet".into()])
            .playlist_id(&url)
            .max_results(50)
            .page_token(&next_page_token)
            .doit()
            .await;

        match result {
            Ok((_response, playlist_item_list_response)) => {
                // Get the song names from the playlist
                extract_songs(&playlist_item_list_response, song_names);
                return get_all_songs_from_pagination_recursive(
                    playlist_item_list_response,
                    song_names,
                    hub,
                    url,
                )
                .await;
            }
            Err(e) => {
                println!("Error: {:?}", e);
                return Some(Err(google_youtube3::Error::from(e)));
            }
        }
    }
    None
}
// Get the list of songs from a youtube playlist url
fn extract_songs(
    playlist_item_list_response: &google_youtube3::api::PlaylistItemListResponse,
    song_names: &mut Vec<String>,
) {
    // Clone playlist_item_list_response
    let playlist_item_list_response = playlist_item_list_response.clone();
    let items = playlist_item_list_response.items.unwrap();

    for playlist_item in items {
        let snippet = playlist_item.clone().snippet.unwrap();
        let title = snippet.title.unwrap();
        // if the video_owner_channel_title is None then the video is deleted
        if snippet.video_owner_channel_title.is_none() {
            continue;
        }
        let channel_name = snippet.video_owner_channel_title.unwrap();

        // if video_owner_channel_title contains "- Topic" then the artist is in video_owner_channel_title
        if channel_name.ends_with("- Topic") {
            let artist = channel_name.split("- Topic").next().unwrap();
            song_names.push(format!("{} - {:?}", artist, title));
        } else {
            song_names.push(title);
        }
    }
}

#[tokio::main]
async fn main() -> google_youtube3::Result<()> {
    let opts: CLIOpts = CLIOpts::parse();

    let download_host = env::var("DOWNLOAD_HOST").unwrap();

    let song_names = get_song_names_from_playlist_url(opts.playlist_id).await;

    match song_names {
        Ok(song_names) => {
            let mut clean_song_names = vec![];
            for song_name in song_names.iter() {
                // let song_name: String = song_name;
                // list of string to remove
                let to_remove_list = [
                    "\"",
                    "(Official Audio)",
                    "(Official Music)",
                    "(Official Video)",
                    "(Official Music Video)",
                    "(Free Download on SoundCloud)",
                    "[Official Audio]",
                    "[Official Video]",
                    "[Official Video 2019]",
                    "(videoclip)",
                    "[OFFICIAL MUSIC VIDEO]",
                    ".wmv",
                    "(Lyric Video)",
                    "(lyrics)",
                    "(Music Video)",
                ];

                // for each to_remove items, remove it from song_name
                let mut clean_song_name: String = song_name.clone();
                for to_remote in to_remove_list {
                    clean_song_name = clean_song_name.replace(to_remote, "");
                }
                clean_song_names.push(clean_song_name.clone());
                println!("{}", &clean_song_name);

                // Get JSON from https://xyz/vk_auth.php?q=<song_name>
                let json_url = format!("{}vk_auth.php?q={}", download_host, clean_song_name);
                println!("{}", &json_url);
                // GET with headers
                let client = reqwest::Client::new();
                let response = client
                    .get(&json_url)
                    .header(
                        reqwest::header::USER_AGENT,
                        "User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/97.0.4692.71 Safari/537.36",
                    )
                    .send()
                    .await
                    .unwrap();

                let json: SliderResult = match serde_json::from_str(&response.text().await.unwrap())
                {
                    Ok(json) => json,
                    Err(e) => {
                        println!("No result, error: {:?}", e);
                        continue;
                    }
                };

                let selected_song = json.audios.field.into_iter().next().unwrap();
                let download_url = format!(
                    "{}download/{}/{}/{}/{}.mp3?extra=null",
                    download_host,
                    selected_song.id,
                    selected_song.duration,
                    selected_song.url,
                    urlencoding::encode(&selected_song.tit_art),
                );

                // Download the song and save it to the current directory
                download_file(
                    &client,
                    &download_url,
                    &format!("download/{}.mp3", clean_song_name),
                )
                .await;
            }

            if opts.output_file.is_some() {
                write_songs_to_file(opts.output_file.unwrap(), clean_song_names)?;
            }
        }
        Err(e) => {
            println!("Error: {:?}", e);
            return Err(google_youtube3::Error::from(e));
        }
    }

    Ok(())
}

// Write clean_song_names into a txt file
fn write_songs_to_file(
    output_file: String,
    clean_song_names: Vec<String>,
) -> google_youtube3::Result<()> {
    let mut file = File::create(output_file)?;
    file.write_all(clean_song_names.join("\n").as_bytes())?;
    Ok(())
}
