use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use askama::Template;
use chrono::DateTime;
use millisecond::Millisecond;
use spools::{Media, MediaKind, Post, Subpost, User};

use crate::{common::error::TimerError, config::Settings, Error, REVISION};

use super::formatters::{body, link, number};

#[derive(Debug, PartialEq)]
enum MediaClosure {
    Start,
    End,
    Single,
}

#[derive(Debug, Template)]
#[template(path = "components/media.j2")]
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
#[template(path = "components/post.j2")]
struct FormattedSubpost<'a> {
    input: Subpost,
    code: Option<&'a str>,
    body: &'a str,
    date: &'a str,
    likes: &'a str,
    media: Vec<String>,
}

trait SubpostRender {
    fn render(&self, preview: bool, base: &Base) -> Result<String, Error>;
}

impl SubpostRender for Subpost {
    fn render(&self, preview: bool, base: &Base) -> Result<String, Error> {
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

        let likes = number(self.likes);

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

        let body = body(&self.body, base)?;

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
    fn render(&self, base: &Base) -> Result<String, Error>;
}

impl PostRender for Post {
    fn render(&self, base: &Base) -> Result<String, Error> {
        let parents = self
            .parents
            .iter()
            .map(|p| p.render(true, base))
            .collect::<Result<String, Error>>()?;

        let replies = self
            .replies
            .iter()
            .map(|p| p.render(true, base))
            .collect::<Result<String, Error>>()?;

        // Rendering already handled by Subpost
        let subpost = Subpost {
            code: String::new(),
            author: self.author.clone(),
            date: self.date,
            body: self.body.clone(),
            media: self.media.clone(),
            likes: self.likes,
        }
        .render(false, base)?;

        Ok(format!("{}{}{}", parents, subpost, replies))
    }
}

/// Common object for base template values
#[derive(Debug, Clone)]
pub struct Base {
    rev: &'static str,
    rss: bool,
    pub(super) url: String,
    time: Option<u128>,
}

impl Base {
    /// Spawns a new Base object
    pub fn new() -> Result<Base, Error> {
        let config = Settings::new()?;

        Ok(Base {
            rev: &REVISION,
            rss: config.endpoint.rss,
            url: config.server.base_url,
            time: None,
        })
    }

    /// Fetches the current time for use in the time function
    fn now() -> Result<u128, SystemTimeError> {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();

        Ok(since_the_epoch)
    }

    /// Formats and displays timer value
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
            Err(Error::Timer(TimerError::NotStarted))
        }
    }

    /// Sets the response time value
    pub fn timer(&mut self, start: bool) -> Result<(), Error> {
        let now = Base::now()?;

        if start {
            self.time = Some(now);
        } else if let Some(time) = self.time {
            if time > now {
                return Err(Error::Timer(TimerError::ClockSkew));
            }

            self.time = Some(now - time);
        } else {
            return Err(Error::Timer(TimerError::NotStarted));
        }

        Ok(())
    }
}

#[derive(Debug, Template)]
#[template(path = "home.j2")]
pub(super) struct HomeView {
    pub(super) base: Base,
}

#[derive(Debug, Template)]
#[template(path = "user.j2")]
pub(super) struct UserView<'a> {
    pub(super) base: Base,
    pub(super) input: &'a str,
    pub(super) output: User,
}

#[derive(Debug, Template)]
#[template(path = "post.j2")]
pub(super) struct PostView<'a> {
    pub(super) base: Base,
    pub(super) input: &'a str,
    pub(super) output: Post,
}

#[derive(Debug, Template)]
#[template(path = "common/error.j2")]
pub struct ErrorView<'a> {
    pub base: Base,
    pub status: &'a str,
    pub error: &'a str,
}
