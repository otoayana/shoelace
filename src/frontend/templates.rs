use std::time::{SystemTime, SystemTimeError, UNIX_EPOCH};

use askama::Template;
use spools::{Media, Post};

use crate::{config::Settings, Error};
use crate::REVISION;

/// Components used by templates which fetch content from Threads
trait Fetched {
    fn suffix(&self, num: u64) -> String;
    fn url(&self, body: String) -> String;
    fn post(&self, post: Post, is_subpost: bool, is_clickable: bool) -> String;
    fn media(&self, media: Media, is_subpost: bool) -> String;
}

/// Common object for base template values
#[derive(Debug)]
pub struct Base {
    rev: &'static str,
    base_url: String,
    time: Option<u128>,
}

impl Base {
    /// Spawns a new Base object
    pub fn new() -> Result<Base, Error> {
        let config = Settings::new()?;
        
        Ok(Base {
            rev: REVISION,
            base_url: config.server.base_url,
            time: None
        })
    }

    /// Fetches the current time for use in the time function
    fn _now() -> Result<u128, SystemTimeError> {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH)?.as_millis();
    
        Ok(since_the_epoch)
    }

    /// Sets the response time value
    pub fn _timer(&mut self, start: bool) -> Result<(), Error> {
        let now = Base::_now()?;

        if !start {
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
pub struct Home {
    pub base: Base
}
