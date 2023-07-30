extern crate clap;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
extern crate oauth_client as oauth;
#[macro_use]
extern crate lazy_static;
extern crate colored;
extern crate mime_guess;
extern crate reqwest;

use chrono::prelude::*;
use clap::{AppSettings, Parser};
use colored::*;
use oauth::{DefaultRequestBuilder, Token};
use reqwest::blocking::multipart::{Form, Part};
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::{BufReader, ErrorKind};
use std::path::Path;
use std::process::Command;
use std::str::FromStr;
use std::sync::RwLock;
use std::thread;

// Account hold information about account
#[allow(dead_code)]
struct PlaceType {
    // name
    name: String,
    // code
    code: f64,
}

#[allow(dead_code)]
struct TrendLocation {
    // name
    name: String,
    // countryCode
    country_code: String,
    // url
    url: String,
    // woeid
    woeid: f64,
    // placeType
    place_type: PlaceType,
    // parentid
    parentid: f64,
    // country
    country: String,
}

#[allow(dead_code)]
struct Timezone {
    // name
    name: String,
    // utc_offset
    utc_offset: f64,
    // tzinfo_name
    tzinfo_name: String,
}

#[allow(dead_code)]
struct SleepTime {
    // enabled
    enabled: bool,
    // end_time
    end_time: String,
    // start_time
    start_time: String,
}

#[allow(dead_code)]
struct Account {
    // time_zone
    time_zone: Timezone,
    // protected
    protected: bool,
    // screen_name
    screen_name: String,
    // always_use_https
    always_use_https: bool,
    // use_cookie_personalization
    use_cookie_personalization: bool,
    // sleep_time
    sleep_time: SleepTime,
    // geo_enabled
    geo_enabled: bool,
    // language
    language: String,
    // discoverable_by_email
    discoverable_by_email: bool,
    // discoverable_by_mobile_phone
    discoverable_by_mobile_phone: bool,
    // display_sensitive_media
    display_sensitive_media: bool,
    // allow_contributor_request
    allow_contributor_request: String,
    // allow_dms_from
    allow_dms_from: String,
    // allow_dm_groups_from
    allow_dm_groups_from: String,
    // smart_mute
    smart_mute: bool,
    // trend_location
    trend_location: Vec<TrendLocation>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct User {
    // name
    name: String,
    // screen_name
    screen_name: String,
    // followers_count
    followers_count: f64,
    // profile_image_url
    profile_image_url: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct Place {
    // id
    id: String,
    // full_name
    full_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct HashTag {
    // indices
    indices: Vec<f64>,
    // text
    text: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct UserMention {
    // indices
    indices: Vec<f64>,
    // screen_name
    screen_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct Url {
    // indices
    indices: Vec<f64>,
    // url
    url: String,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct Entities {
    hashtags: Vec<HashTag>,
    user_mentions: Vec<UserMention>,
    urls: Vec<Url>,
}

// Tweet hold information about tweet
#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
struct Tweet {
    // text
    text: String,
    // id_str
    id_str: String,
    // source
    source: String,
    // created_at
    created_at: String,
    // user
    user: User,
    // place
    // place: Place,
    // entities
    entities: Entities,
}

// SearchMetadata hold information about search metadata
#[allow(dead_code)]
struct SearchMetadata {
    // completed_in
    completed_in: f64,
    // max_id
    max_id: f64,
    // max_id_str
    max_id_str: String,
    // next_results
    next_results: String,
    // query
    query: String,
    // refresh_url
    refresh_url: String,
    // count
    count: f64,
    // since_id
    since_id: f64,
    // since_id_str
    since_id_str: String,
}

#[allow(dead_code)]
struct Item {
    title: String,
    description: String,
    pubdate: String,
    links: Vec<String>,
    guid: String,
    author: String,
}

#[allow(dead_code)]
struct Channel {
    title: String,
    description: String,
    link: String,
    items: Vec<Item>,
}

// Rss hold information about RSS
#[allow(dead_code)]
struct Rss {
    channel: Channel,
}

struct Config {
    file: String,
    credential: serde_json::Value,
}

impl Config {
    pub fn new() -> Config {
        Config {
            file: "".to_owned(),
            credential: json!(null),
        }
    }
}

#[derive(Parser, Debug)]
#[clap(about, version, author, override_usage = "rstw [FLAGS] [OPTIONS] [TEXT]")]
#[clap(setting(AppSettings::AllowExternalSubcommands))]
struct ArgsForClap {
    #[clap(
        short = 'a',
        long,
        value_name = "ACCOUNT",
        help = "switch account to load configuration file. Note: experimental"
    )]
    account: Option<String>,

    #[clap(short = 'f', long, value_name = "ID", help = "specify favorite ID")]
    fav_id: Option<String>,

    #[clap(
        short = 'i',
        long,
        value_name = "ID",
        help = "specify in-reply ID, if not specify text, it will be RT."
    )]
    inreply_id: Option<String>,

    #[clap(
        short = 'l',
        long,
        value_name = "USER/LIST",
        help = "show list's timeline (ex: mattn_jp/subtech)"
    )]
    list: Option<String>,

    #[clap(short = 'm', value_name = "FILE", long, help = "upload media")]
    media: Option<Vec<String>>,

    #[clap(short = 'u', value_name = "USER", long, help = "show user's timeline")]
    user: Option<String>,

    #[clap(short = 's', long, value_name = "WORD", help = "search timeline")]
    query: Option<String>,

    #[clap(short = 'j', long, help = "as JSON")]
    asjson: bool,

    #[clap(short = 'S', long, help = "stream timeline")]
    stream: bool,

    #[clap(short = 'r', long, help = "show replies")]
    reply: bool,

    #[clap(
        short = 'F',
        long,
        value_name = "FILENAME",
        help = "post utf-8 string from a file(\"-\" means STDIN)"
    )]
    from_file: Option<String>,

    #[clap(short = 'c', long, value_name = "NUMBER", help = "show NUMBER tweets at timeline.")]
    count: Option<String>,

    #[clap(
        long,
        value_name = "DATE",
        help = "show tweets created after the DATE (ex. 2017-05-01)"
    )]
    since: Option<String>,

    #[clap(
        long,
        value_name = "DATE",
        help = "show tweets created before the DATE (ex. 2017-05-31)"
    )]
    until: Option<String>,

    #[clap(long, value_name = "NUMBER", help = "show tweets that have ids greater than NUMBER.")]
    since_id: Option<String>,

    #[clap(long, value_name = "NUMBER", help = "show tweets that have ids lower than NUMBER.")]
    max_id: Option<String>,

    #[clap(short = 'v', long, help = "show detail")]
    verbose: bool,

    #[clap(short = 'd', long, help = "show raw json")]
    debug: bool,

    #[clap(subcommand)]
    command: Option<ExtCommand>,
}

#[derive(Parser, Debug)]
enum ExtCommand {
    #[clap(external_subcommand)]
    Other(Vec<String>),
}

struct Args {
    account: String,
    fav_id: String,
    inreply_id: String,
    list: String,
    media: Vec<String>,
    user: String,
    query: String,
    asjson: bool,
    stream: bool,
    reply: bool,
    from_file: String,
    count: String,
    since: String,
    until: String,
    since_id: i64,
    max_id: i64,
    verbose: bool,
    debug: bool,
}

impl Args {
    pub fn new() -> Args {
        Args {
            account: "".to_owned(),
            fav_id: "".to_owned(),
            inreply_id: "".to_owned(),
            list: "".to_owned(),
            media: Vec::new(),
            user: "".to_owned(),
            query: "".to_owned(),
            asjson: false,
            stream: false,
            reply: false,
            from_file: "".to_owned(),
            count: "".to_owned(),
            since: "".to_owned(),
            until: "".to_owned(),
            since_id: 0,
            max_id: 0,
            verbose: false,
            debug: false,
        }
    }
}

lazy_static! {
    static ref ARGS: RwLock<Args> = RwLock::new(Args::new());
    static ref CONF: RwLock<Config> = RwLock::new(Config::new());
}

macro_rules! ARGSR {
    ($name: ident) => {
        ARGS.read().unwrap().$name
    };
}

macro_rules! ARGSW {
    ($name: ident) => {
        ARGS.write().unwrap().$name
    };
}

macro_rules! CONFR {
    ($name: ident) => {
        CONF.read().unwrap().$name
    };
}

macro_rules! CONFW {
    ($name: ident) => {
        CONF.write().unwrap().$name
    };
}

const TIME_FMT: &str = "%a %b %e %T %z %Y";

fn to_local_time(value: &str) -> String {
    match NaiveDateTime::parse_from_str(value, TIME_FMT) {
        Ok(tm) => {
            let local: DateTime<Local> = Local.from_local_datetime(&tm).unwrap();
            local.format(TIME_FMT).to_string()
        }
        Err(err) => {
            println!("failed to parse time string. reason: {}", err);
            value.to_owned()
        }
    }
}

fn show_tweets(tweets: &[Tweet], verbose: bool) {
    if ARGSR!(asjson) {
        for tweet in tweets.iter().rev() {
            println!("{}", serde_json::to_string(&tweet).unwrap());
        }
    } else if verbose {
        for tweet in tweets.iter().rev() {
            println!("{}: {}", tweet.user.screen_name.red(), tweet.user.name.red());
            println!(
                "  {}",
                tweet.text.replace("\r", "").replace("\n", " ").replace("\t", " ",)
            );
            println!("  {}", tweet.id_str);
            println!("  {}", to_local_time(&tweet.created_at));
            // println!("  {}", tweet.created_at);
            println!();
        }
    } else {
        for tweet in tweets.iter().rev() {
            println!("{}: {}", tweet.user.screen_name.red(), tweet.text);
        }
    }
}

fn split_query(query: &str) -> HashMap<Cow<str>, Cow<str>> {
    let mut param = HashMap::new();
    for q in query.split('&') {
        let mut s = q.splitn(2, '=');
        let k = s.next().unwrap();
        let v = s.next().unwrap();
        let _ = param.insert(k.into(), v.into());
    }
    param
}

fn get_request_token(consumer: &Token) -> Token<'static> {
    let (header, _) = oauth::authorization_header(
        "GET",
        "https://api.twitter.com/oauth/request_token",
        consumer,
        None,
        None,
    );
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&header).unwrap());
    let mut response = client
            .get("https://api.twitter.com/oauth/request_token")
            // .unwrap()
            .headers(headers)
            .send()
            .unwrap();
    let mut resp = String::new();
    let _ = response.read_to_string(&mut resp).unwrap();
    let param = split_query(resp.as_ref());
    Token::new(
        param.get("oauth_token").unwrap().to_string(),
        param.get("oauth_token_secret").unwrap().to_string(),
    )
}

fn get_access_token(consumer: &Token, request: &Token, pin: &str) -> Token<'static> {
    let mut params = HashMap::new();
    params.insert("oauth_verifier".into(), pin.into());
    let (header, body) = oauth::authorization_header(
        "GET",
        "https://api.twitter.com/oauth/access_token",
        consumer,
        Some(request),
        Some(&params),
    );
    let client = Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(reqwest::header::AUTHORIZATION, HeaderValue::from_str(&header).unwrap());
    let mut res = client
        .get(&format!(
            "https://api.twitter.com/oauth/access_token?{}",
            body
        ))
        // .unwrap()
        .headers(headers)
        .send()
        .unwrap();
    let mut resp = String::new();
    let _ = res.read_to_string(&mut resp).unwrap();
    let param = split_query(resp.as_ref());
    Token::new(
        param.get("oauth_token").unwrap().to_string(),
        param.get("oauth_token_secret").unwrap().to_string(),
    )
}

fn get_token() -> bool {
    if CONFR!(credential)["access_key"].is_null() {
        let consumer_key = CONFR!(credential)["consumer_key"].as_str().unwrap().to_string();
        let consumer_secret = CONFR!(credential)["consumer_secret"].as_str().unwrap().to_string();
        let consumer = Token::new(consumer_key, consumer_secret);
        let request = get_request_token(&consumer);

        let url = format!("https://api.twitter.com/oauth/authorize?oauth_token={}", request.key);

        let browser: String;
        let args: Vec<String>;
        if env::consts::OS == "windows" {
            browser = "rundll.exe".to_owned();
            args = vec!["url.dll,FileProtocolHandler".to_owned(), url.clone()];
        } else if env::consts::OS == "macos" {
            browser = "open".to_owned();
            args = vec![url.clone()];
        } else {
            browser = "open".to_owned();
            args = vec![url.clone()];
        }

        Command::new(browser)
            .args(&args)
            .spawn()
            .expect("failed to start command");

        println!("{}", "Open this URL and enter PIN.".red());
        println!("  {}", url);
        println!("PIN:");
        let mut pin = String::new();
        io::stdin().read_line(&mut pin).unwrap();

        let access: Token = get_access_token(&consumer, &request, &pin);
        CONFW!(credential)["access_key"] = json!(access.key);
        CONFW!(credential)["access_secret"] = json!(access.secret);
        true
    } else {
        false
    }
}

fn read_config() {
    let mut dir: String = match env::var("HOME") {
        Ok(val) => val,
        Err(_) => "".to_owned(),
    };

    if dir.is_empty() && env::consts::OS == "windows" {
        dir = match env::var("APPDATA") {
            Ok(app_dir) => String::from(Path::new(&app_dir).join("rstw").to_str().unwrap()),
            Err(_) => match env::var("USERPROFILE") {
                Ok(prof_dir) => String::from(
                    Path::new(&prof_dir)
                        .join("Application Data")
                        .join("rstw")
                        .to_str()
                        .unwrap(),
                ),
                Err(_) => panic!("%USERPROFILE% path not found."),
            },
        };
    } else {
        dir = String::from(Path::new(&dir).join(".config").join("rstw").to_str().unwrap());
    }

    match fs::create_dir_all(&dir) {
        Ok(_) => {}
        Err(err) => {
            panic!("could not create config dirctory. reason: {}", err);
        }
    };

    let file_path: String = if ARGSR!(account).is_empty() {
        String::from(Path::new(&dir).join("settings.json").to_str().unwrap())
    } else {
        let path = String::from(
            Path::new(&dir)
                .join(ARGSR!(account).to_string() + ".json")
                .to_str()
                .unwrap(),
        );
        if !Path::new(&path).exists() {
            panic!("could not find config file {}", path);
        }
        path
    };

    let mut credential: serde_json::Value = json!(null);
    match File::open(&file_path) {
        Ok(mut file) => {
            let mut file_data = String::new();
            match file.read_to_string(&mut file_data) {
                Ok(_) => {}
                Err(err) => panic!("failed to read config file. reason: {}", err),
            }

            credential = match serde_json::from_str(&file_data) {
                Ok(val) => val,
                Err(err) => panic!("failed to parse config file. reason: {}", err),
            }
        }
        Err(err) => {
            if err.kind() == ErrorKind::NotFound {
                credential["consumer_key"] = json!("xxxx");
                credential["consumer_secret"] = json!("xxxx");
            } else {
                panic!("failed to open config file. reason: {}", err)
            }
        }
    };

    CONFW!(file) = file_path;
    CONFW!(credential) = credential;
}

fn read_file(file_path: &str) -> String {
    let mut file_data = String::new();

    if file_path == "-" {
        io::stdin().read_line(&mut file_data).unwrap();
    } else {
        let mut file = match File::open(file_path) {
            Ok(file) => file,
            Err(err) => panic!("failed to open the file. reason: {}", err),
        };

        match file.read_to_string(&mut file_data) {
            Ok(_) => {}
            Err(err) => panic!("failed to read the file. reason: {}", err),
        };
    }
    file_data
}

fn save_credential() {
    let mut file = match File::create(&CONFR!(file)) {
        Ok(file) => file,
        Err(err) => panic!("failed to create the file. reason: {}", err),
    };

    match file.write_all(CONFR!(credential).to_string().as_bytes()) {
        Ok(_) => {}
        Err(err) => panic!("failed to write to the file. reason: {}", err),
    };
}

fn count_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>) {
    if ARGSR!(count).parse::<i64>().is_ok() {
        param.insert("count".into(), ARGSR!(count).clone().into());
    }
}

fn since_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>) {
    timeformat_to_param(param, "since", &ARGSR!(since));
}

fn until_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>) {
    timeformat_to_param(param, "until", &ARGSR!(until));
}

fn timeformat_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>, name: &str, value: &str) {
    if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
        param.insert(name.to_string().into(), value.to_string().into());
    }
}

fn sinceid_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>) {
    id_to_param(param, "since_id", ARGSR!(since_id));
}

fn maxid_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>) {
    id_to_param(param, "max_id", ARGSR!(max_id));
}

fn id_to_param<'a>(param: &mut HashMap<Cow<'a, str>, Cow<'a, str>>, name: &str, value: i64) {
    if value > 0 {
        param.insert(name.to_string().into(), format!("{}", value).into());
    }
}

fn upload(file_name: &str, access: &Token, consumer: &Token) -> String {
    if !Path::new(file_name).exists() {
        panic!("could not read media file.");
    }

    let mime_type = mime_guess::from_path(file_name).first_raw().unwrap_or("");

    // INIT
    let metadata = fs::metadata(file_name).unwrap();
    let mut param = HashMap::new();
    param.insert("command".into(), "INIT".into());
    param.insert("media_type".into(), mime_type.into());
    param.insert("total_bytes".into(), metadata.len().to_string().into());

    let client = Client::new();
    let mut headers = HeaderMap::new();
    let (header, _) = oauth::authorization_header(
        "POST",
        "https://upload.twitter.com/1.1/media/upload.json",
        consumer,
        Some(access),
        None,
    );

    headers.insert(AUTHORIZATION, HeaderValue::from_str(&header).unwrap());
    match oauth::post::<DefaultRequestBuilder>(
        "https://upload.twitter.com/1.1/media/upload.json",
        consumer,
        Some(access),
        Some(&param),
        &(),
    ) {
        Ok(bytes) => {
            let value: serde_json::Value = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();

            let mut file = File::open(file_name).unwrap();
            let mut buf: Vec<u8> = Vec::new();
            file.read_to_end(&mut buf).unwrap();

            // APPEND
            // TODO: separate 5MB chunk
            let part = Part::bytes(buf);
            let media_id_string = value["media_id"].to_string();
            let form = Form::new()
                .text("command", "APPEND")
                .text("media_id", media_id_string.to_string())
                .part("media", part)
                .text("segment_index", "0");

            headers.insert(
                "Content-Type",
                HeaderValue::from_str(&format!("multipart/form-data; boundary={}", form.boundary())).unwrap(),
            );

            match client
                .post("https://upload.twitter.com/1.1/media/upload.json")
                .headers(headers)
                .multipart(form)
                .send()
            {
                Ok(_) => {
                    // FINALIZE
                    param.clear();
                    param.insert("command".into(), "FINALIZE".into());
                    param.insert("media_id".into(), media_id_string.into());
                    match oauth::post::<DefaultRequestBuilder>(
                        "https://upload.twitter.com/1.1/media/upload.json",
                        consumer,
                        Some(access),
                        Some(&param),
                        &(),
                    ) {
                        Ok(bytes) => {
                            let value: serde_json::Value =
                                serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                            value["media_id"].to_string()
                        }
                        Err(err) => panic!("failed to FINALIZE: {}", err),
                    }
                }
                Err(err) => panic!("filed to APPEND: {}", err),
            }
        }
        Err(err) => panic!("failed to INIT: {}", err),
    }
}

fn main() {
    let arg_clap = ArgsForClap::parse();

    if let Some(v) = arg_clap.account {
        ARGSW!(account) = v;
    }

    if let Ok(v) = env::var("RSTW_ACCOUNT") {
        ARGSW!(account) = v;
    }

    if let Some(v) = arg_clap.fav_id {
        ARGSW!(fav_id) = v;
    }

    if let Some(v) = arg_clap.inreply_id {
        ARGSW!(inreply_id) = v;
    }

    if let Some(v) = arg_clap.list {
        ARGSW!(list) = v;
    }

    if let Some(v) = arg_clap.media {
        ARGSW!(media) = v;
    }

    if let Some(v) = arg_clap.user {
        ARGSW!(user) = v;
    }

    if let Some(v) = arg_clap.query {
        ARGSW!(query) = v;
    }

    ARGSW!(asjson) = arg_clap.asjson;
    ARGSW!(stream) = arg_clap.stream;
    ARGSW!(reply) = arg_clap.reply;

    if let Some(v) = arg_clap.from_file {
        ARGSW!(from_file) = v;
    }

    if let Some(v) = arg_clap.count {
        ARGSW!(count) = v;
    }

    if let Some(v) = arg_clap.since {
        ARGSW!(since) = v;
    }

    if let Some(v) = arg_clap.until {
        ARGSW!(until) = v;
    }

    if let Some(v) = arg_clap.since_id {
        if let Ok(num) = i64::from_str(&v) {
            ARGSW!(since_id) = num;
        }
    }

    if let Some(v) = arg_clap.max_id {
        if let Ok(num) = i64::from_str(&v) {
            ARGSW!(max_id) = num;
        }
    }

    ARGSW!(verbose) = arg_clap.verbose;
    ARGSW!(debug) = arg_clap.debug;

    read_config();

    if get_token() {
        save_credential();
    }

    let consumer_key = CONFR!(credential)["consumer_key"].as_str().unwrap().to_string();
    let consumer_secret = CONFR!(credential)["consumer_secret"].as_str().unwrap().to_string();

    let consumer: Token = Token::new(consumer_key, consumer_secret);

    let access_key = CONFR!(credential)["access_key"].as_str().unwrap().to_string();
    let access_secret = CONFR!(credential)["access_secret"].as_str().unwrap().to_string();

    let access: Token = Token::new(access_key, access_secret);

    let mut param = HashMap::new();
    if !ARGSR!(media).is_empty() {
        let mut media_ids: Vec<String> = Vec::new();
        ARGS.read()
            .unwrap()
            .media
            .iter()
            .for_each(|media| media_ids.push(upload(media, &access, &consumer)));

        // Unknown subcommand may be tweet contents.
        if let Some(ext_cmd) = arg_clap.command {
            match ext_cmd {
                ExtCommand::Other(contents) => {
                    param.clear();
                    param.insert("status".into(), contents.join(" ").into());
                    param.insert("in_reply_to_status_id".into(), ARGSR!(inreply_id).clone().into());
                    param.insert("media_ids".into(), media_ids.join(",").into());
                    match oauth::post::<DefaultRequestBuilder>(
                        "https://api.twitter.com/1.1/statuses/update.json",
                        &consumer,
                        Some(&access),
                        Some(&param),
                        &(),
                    ) {
                        Ok(bytes) => {
                            let tweet: Tweet = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                            println!("tweeted: {}", tweet.id_str);
                        }
                        Err(err) => println!("failed to post tweet: {}", err),
                    }
                }
            }
        }

        return;
    }

    if !ARGSR!(query).is_empty() {
        param.insert("q".into(), ARGSR!(query).clone().into());
        count_to_param(&mut param);
        since_to_param(&mut param);
        until_to_param(&mut param);
        match oauth::get::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/search/tweets.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let res: serde_json::Value = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                let val_vec: &Vec<serde_json::Value> = res["statuses"].as_array().unwrap();
                let tweets: Vec<Tweet> = val_vec
                    .iter()
                    .map(|val| serde_json::from_value(val.clone()).unwrap())
                    .collect();
                show_tweets(&tweets, ARGSR!(verbose));
            }
            Err(err) => println!("failed to get statuses: {}", err),
        }
    } else if ARGSR!(reply) {
        count_to_param(&mut param);
        match oauth::get::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/statuses/mentions_timeline.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let tweets: Vec<Tweet> = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                show_tweets(&tweets, ARGSR!(verbose));
            }
            Err(err) => println!("failed to get tweets: {}", err),
        }
    } else if !ARGSR!(list).is_empty() {
        let part_str: String = ARGSR!(list).clone();
        let part_vec: Vec<&str> = part_str.splitn(2, '/').collect();
        if part_vec.len() == 1 {
            match oauth::get::<DefaultRequestBuilder>(
                "https://api.twitter.com/1.1/account/settings.json",
                &consumer,
                Some(&access),
                None,
                &(),
            ) {
                Ok(bytes) => {
                    let res: serde_json::Value =
                        serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                    param.insert(
                        "owner_screen_name".into(),
                        res["screen_name"].as_str().unwrap().to_string().into(),
                    );
                    param.insert("slug".into(), part_vec[0].to_string().into());
                }
                Err(err) => {
                    println!("failed to get account: {}", err);
                    return;
                }
            }
        } else {
            param.insert("owner_screen_name".into(), part_vec[0].to_string().into());
            param.insert("slug".into(), part_vec[1].to_string().into());
        }

        count_to_param(&mut param);
        sinceid_to_param(&mut param);
        maxid_to_param(&mut param);

        match oauth::get::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/lists/statuses.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let tweets: Vec<Tweet> = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                show_tweets(&tweets, ARGSR!(verbose));
            }
            Err(err) => println!("failed to get tweets: {}", err),
        }
    } else if !ARGSR!(user).is_empty() {
        param.insert("screen_name".into(), ARGSR!(user).clone().into());
        count_to_param(&mut param);
        sinceid_to_param(&mut param);
        maxid_to_param(&mut param);
        match oauth::get::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/statuses/user_timeline.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let tweets: Vec<Tweet> = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                show_tweets(&tweets, ARGSR!(verbose));
            }
            Err(err) => println!("failed to get tweets: {}", err),
        }
    } else if !ARGSR!(fav_id).is_empty() {
        param.insert("id".into(), ARGSR!(fav_id).clone().into());
        match oauth::post::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/favorites/create.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(_) => {
                print!("{}", "\u{2764}".red());
                println!("favorited");
            }
            Err(err) => println!("failed to create favorite: {}", err),
        }
    } else if ARGSR!(stream) {
        let client = Client::new();
        let mut headers = HeaderMap::new();
        let (header, _) = oauth::authorization_header(
            "GET",
            "https://userstream.twitter.com/1.1/user.json",
            &consumer,
            Some(&access),
            None,
        );
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&header).unwrap());
        let res = client
            .get("https://userstream.twitter.com/1.1/user.json")
            // .unwrap()
            .headers(headers)
            .send()
            .unwrap();
        let receive_loop = thread::spawn(move || {
            let buf = BufReader::new(res);
            for line in buf.lines() {
                let line_str = line.unwrap();
                let val: serde_json::Value = match serde_json::from_str(&line_str) {
                    Ok(val) => val,
                    Err(_) => continue,
                };
                if !val["id_str"].is_null() {
                    let tweet: Tweet = serde_json::from_str(&line_str).unwrap();
                    show_tweets(&vec![tweet], ARGSR!(verbose));
                }
            }
        });
        let _ = receive_loop.join();
    } else if !ARGSR!(from_file).is_empty() {
        let text = read_file(&ARGSR!(from_file));
        param.insert("status".into(), text.into());
        param.insert("in_reply_to_status_id".into(), ARGSR!(inreply_id).clone().into());
        match oauth::post::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/statuses/update.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let tweet: Tweet = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                println!("tweeted: {}", tweet.id_str);
            }
            Err(err) => println!("failed to post tweet: {}", err),
        }
    } else if env::args().len() == 1 {
        count_to_param(&mut param);
        match oauth::get::<DefaultRequestBuilder>(
            "https://api.twitter.com/1.1/statuses/home_timeline.json",
            &consumer,
            Some(&access),
            Some(&param),
            &(),
        ) {
            Ok(bytes) => {
                let tweets: Vec<Tweet> = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                show_tweets(&tweets, ARGSR!(verbose));
            }
            Err(err) => println!("failed to get tweet: {}", err),
        }
    } else {
        // Unknown subcommand may be tweet contents.
        match arg_clap.command {
            Some(ext_cmd) => match ext_cmd {
                ExtCommand::Other(contents) => {
                    param.insert("status".into(), contents.join(" ").into());
                    param.insert("in_reply_to_status_id".into(), ARGSR!(inreply_id).clone().into());
                    match oauth::post::<DefaultRequestBuilder>(
                        "https://api.twitter.com/1.1/statuses/update.json",
                        &consumer,
                        Some(&access),
                        Some(&param),
                        &(),
                    ) {
                        Ok(bytes) => {
                            let tweet: Tweet = serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                            println!("tweeted: {}", tweet.id_str);
                        }
                        Err(err) => println!("failed to post tweet: {}", err),
                    }
                }
            },
            _ => {
                count_to_param(&mut param);
                match oauth::get::<DefaultRequestBuilder>(
                    "https://api.twitter.com/1.1/statuses/home_timeline.json",
                    &consumer,
                    Some(&access),
                    Some(&param),
                    &(),
                ) {
                    Ok(bytes) => {
                        let tweets: Vec<Tweet> =
                            serde_json::from_str(&String::from_utf8(bytes.into()).unwrap()).unwrap();
                        show_tweets(&tweets, ARGSR!(verbose));
                    }
                    Err(err) => println!("failed to get tweet: {}", err),
                }
            }
        }
    }
}
