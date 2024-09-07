use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use askama::Template;
use chrono::DateTime;
use numfmt::{Formatter, Precision, Scales};
use spools::{Post, Subpost, User};

use crate::{config::Settings, Error};
use crate::REVISION;

fn common_fmt<'a>(value: u64) -> String {
    let mut formatter = Formatter::new()
        .scales(Scales::short())
        .precision(Precision::Significance(2));
    let format = formatter.fmt2(value);

    format.to_owned()
}

#[derive(Debug, Template)]
#[template(path = "components/post.html")]
struct FormattedSubpost<'a> {
    code: Option<&'a str>,
    // The compiler doesn't recognize Askama will use these lol
    #[allow(dead_code)]
    date: &'a str,
    #[allow(dead_code)]
    likes: &'a str,
    input: Subpost,
}

trait BlockExtension {
    fn render(&self) -> Result<String, Error>;
}

impl BlockExtension for Subpost {
    fn render(&self) -> Result<String, Error> {
        /*
        Subposts are recognized passively, by detecting the prescence
        of a code ID, and matching an Option value within the template.
        */
        let mut code: Option<&str> = None;

        if self.code.len() > 0 {
            code = Some(&self.code)
        }

        let date = if let Some(date) = DateTime::from_timestamp(self.date as i64, 0) {
            date.format("%Y-%m-%d").to_string()
        } else {
            String::new()
        };

        let likes = common_fmt(self.likes);

        // TODO(otoayana): add media rendering
        
        let template = FormattedSubpost { code, date: date.as_str(), likes: &likes, input: self.clone() };
        Ok(format!("{}", template.render()?))
    } 
}

impl BlockExtension for Post {
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

        Ok(subpost.render()?)
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
            time: None
        })
    }

    /// Fetches the current time for use in the time function
    fn now() -> Result<u128, SystemTimeError> {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();
    
        Ok(since_the_epoch)
    }

    /// Sets the response time value
    pub fn timer(&mut self, start: bool) -> Result<(), Error> {
        let now = Base::now()?;

        if start {
            self.time = Some(now);
        } else {
            if let Some(time) = self.time {
                if time > now {
                    // TODO(lux): make this error more idiomatic
                    return Err(Error::NotFound)
                }

                self.time = Some(now - time);
            } else {
                return Err(Error::NotFound)
            }
        }

        Ok(())
    }
}

#[derive(Debug, Template)]
#[template(path = "home.html")]
pub struct HomeView {
    pub base: Base
}


trait UserUtils {
    fn link_format(link: String) -> String;
}

#[derive(Debug, Template)]
#[template(path = "user.html", print = "all")]
pub struct UserView<'a> {
    pub base: Base,
    pub input: &'a str,
    pub output: User
}

impl<'a> UserUtils for UserView<'a> {
    fn link_format(link: String) -> String {
        format!("<a href=\"{}\">{}</a>", &link, &link.trim_start_matches("http://").trim_start_matches("https://").trim_end_matches("/").to_string())
    }
}
