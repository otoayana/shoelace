use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use askama::Template;
use chrono::DateTime;
use linkify::LinkFinder;
use millisecond::Millisecond;
use numfmt::{Formatter, Precision, Scales};
use spools::{Media, MediaKind, Post, Subpost, User};

use crate::REVISION;
use crate::{config::Settings, Error};

fn common_fmt(value: u64) -> String {
    let mut formatter = Formatter::new()
        .scales(Scales::short())
        .precision(Precision::Significance(2));
    let format = formatter.fmt2(value);

    format.to_owned()
}

#[derive(Debug, PartialEq)]
enum MediaClosure {
    Start,
    End,
    Single,
}

#[derive(Debug, Template)]
#[template(path = "components/media.html")]
struct FormattedMedia<'a> {
    input: Media,
    alt: &'a str,
    preview: bool,
    closure: MediaClosure,
}

trait MediaRender {
    fn render(&self, preview: bool, index: usize, length: usize) -> Result<String, Error>;
}

impl<'a> MediaRender for Media {
    fn render(&self, preview: bool, index: usize, length: usize) -> Result<String, Error> {
        let closure: MediaClosure;

        if index % 2 == 0 && index != length - 1 {
            closure = MediaClosure::Start
        } else if index % 2 != 0 {
            closure = MediaClosure::End
        } else {
            closure = MediaClosure::Single
        }

        let alt: &'a str = if let Some(alt) = self.alt.clone() {
            Box::leak(alt.into_boxed_str())
        } else {
            ""
        };

        let template = FormattedMedia {
            input: self.clone(),
            alt,
            preview,
            closure,
        };

        Ok(template.render()?)
    }
}

#[derive(Debug, Template)]
#[template(path = "components/post.html")]
struct FormattedSubpost<'a> {
    input: Subpost,
    code: Option<&'a str>,
    body: &'a str,
    date: &'a str,
    likes: &'a str,
    media: Vec<String>,
}

trait SubpostRender {
    fn render(&self, preview: bool) -> Result<String, Error>;
}

impl SubpostRender for Subpost {
    fn render(&self, preview: bool) -> Result<String, Error> {
        /*
        Subposts are recognized passively, by detecting the prescence
        of a code ID, and matching an Option value within the template.
        */
        let mut code: Option<&str> = None;

        if !self.code.is_empty() {
            code = Some(&self.code)
        }

        let date = if let Some(date) = DateTime::from_timestamp(self.date as i64, 0) {
            date.format("%Y-%m-%d").to_string()
        } else {
            String::new()
        };

        let likes = common_fmt(self.likes);

        let media_length = self.media.len();
        let mut media_cursor = 0;

        let media = self
            .media
            .clone()
            .iter()
            .map(|o| {
                let render = o.render(preview, media_cursor, media_length);
                media_cursor += 1;
                render
            })
            .collect::<Result<Vec<String>, Error>>();

        // TODO(otoayana): find and link mentions
        let mut body = self.body.clone();
        let mut offset: usize = 0;
        let finder = LinkFinder::new();

        finder.links(&self.body).for_each(|l| {
            // TODO(otoayana): trim URI prefix
            let start_tag = format!("<a href=\"{}\">", l.as_str());

            body.insert_str(l.start() + offset, &start_tag);
            offset += start_tag.len();
            body.insert_str(l.end() + offset, "</a>");
            offset += 4;
        });

        let template = FormattedSubpost {
            input: self.clone(),
            code,
            date: date.as_str(),
            body: &body,
            likes: &likes,
            media: media?,
        };
        Ok((template.render()?).to_string())
    }
}

trait PostRender {
    fn render(&self) -> Result<String, Error>;
}

impl PostRender for Post {
    fn render(&self) -> Result<String, Error> {
        // Rendering already handled by Subpost
        let subpost = Subpost {
            code: String::new(),
            author: self.author.clone(),
            date: self.date,
            body: self.body.clone(),
            media: self.media.clone(),
            likes: self.likes,
        };

        subpost.render(false)
    }
}

/// Common object for base template values
#[derive(Debug, Clone)]
pub struct Base {
    rev: &'static str,
    rss: bool,
    base_url: String,
    time: Option<u128>,
}

impl Base {
    /// Spawns a new Base object
    pub fn new() -> Result<Base, Error> {
        let config = Settings::new()?;

        Ok(Base {
            rev: &REVISION,
            rss: config.endpoint.rss,
            base_url: config.server.base_url,
            time: None,
        })
    }

    /// Fetches the current time for use in the time function
    fn now() -> Result<u128, SystemTimeError> {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();

        Ok(since_the_epoch)
    }

    fn display_timer(&self) -> Result<String, Error> {
        if let Some(time) = self.time {
            let millis = Millisecond::from_millis(time);
            Ok(if millis.seconds == 0 {
                format!("{}ms", millis.millis)
            } else {
                format!(
                    "{:.2}s",
                    millis.seconds as f64 + millis.millis as f64 / 1000.0
                )
            })
        } else {
            // TODO(otoayana): make this error more idiomatic
            Err(Error::NotFound)
        }
    }

    /// Sets the response time value
    pub fn timer(&mut self, start: bool) -> Result<(), Error> {
        let now = Base::now()?;

        if start {
            self.time = Some(now);
        } else if let Some(time) = self.time {
            if time > now {
                // TODO(otoayana): make this error more idiomatic
                return Err(Error::NotFound);
            }

            self.time = Some(now - time);
        } else {
            return Err(Error::NotFound);
        }

        Ok(())
    }
}

#[derive(Debug, Template)]
#[template(path = "home.html")]
pub struct HomeView {
    pub base: Base,
}

trait UserUtils {
    fn link_format(link: String) -> String;
}

#[derive(Debug, Template)]
#[template(path = "user.html")]
pub struct UserView<'a> {
    pub base: Base,
    pub input: &'a str,
    pub output: User,
}

impl<'a> UserUtils for UserView<'a> {
    fn link_format(link: String) -> String {
        format!(
            "<a href=\"{}\">{}</a>",
            &link,
            &link
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/')
                .to_string()
        )
    }
}
