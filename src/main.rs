use google_youtube3::YouTube;
use google_youtube3::{Error, Result};
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

// Get the list of songs from a youtube playlist url
async fn get_song_names_from_playlist_url(url: &str) -> Result<Vec<String>> {
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

    // let scopes = &["https://www.googleapis.com/auth/youtube.readonly"];

    // // token(<scopes>) is the one important function of this crate; it does everything to
    // // obtain a token that can be sent e.g. as Bearer token.
    // match auth.token(scopes).await {
    //     Ok(token) => println!("The token is {:?}", token),
    //     Err(e) => println!("error: {:?}", e),
    // }

    // let https = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots();

    let hub = YouTube::new(
        hyper::Client::builder().build(hyper_rustls::HttpsConnector::with_native_roots()),
        // hyper::Client::builder().build(https),
        auth,
    );

    let result = hub
        .playlist_items()
        .list(&vec!["snippet".into()])
        .playlist_id(url)
        .doit()
        .await;
    println!("result {:?}", result);

    match result {
        Ok(response) => {
            println!("Response {:?}", response);
            let song_names = Vec::new();
            Ok(song_names)
            // for item in response.into() {
            //     let item: serde_json::Value = serde_json::from_str(&item).unwrap();
            //     song_names.push(item.snippet.title.clone());
            // }
            // Ok(song_names)
        }
        Err(e) => Err(Error::from(e)),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    get_song_names_from_playlist_url("PLdCEOoJn13QhC1dNF_b0in7vmg8UBYA7X").await;

    Ok(())
}
