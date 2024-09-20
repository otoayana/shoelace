use linkify::LinkFinder;
use numfmt::{Formatter, Precision, Scales};
use regex::Regex;

use crate::{frontend::templates::Base, Error};

pub(super) fn link(link: &str) -> String {
    format!(
        "<a href=\"{}\">{}</a>",
        link,
        link.trim_start_matches("http://")
            .trim_start_matches("https://")
            .trim_end_matches('/')
    )
}

pub(super) fn number(value: u64) -> String {
    let format: String = if value >= 10 {
        let mut formatter = Formatter::new()
            .scales(Scales::short())
            .precision(Precision::Significance(2));

        formatter
            .fmt2(value)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
    } else {
        format!("{}", value)
    };

    format.to_owned()
}

pub(super) fn body(body: &str, base: &Base) -> Result<String, Error> {
    let mut inner_body = body.to_string();
    let mut offset: isize = 0;
    let finder = LinkFinder::new();

    finder.links(inner_body.clone().as_str()).for_each(|l| {
        let left = &inner_body[..(l.start() as isize + offset) as usize];
        let right = &inner_body[(l.end() as isize + offset) as usize..];
        let text = l.as_str();

        let link = link(text);
        offset += link.clone().len() as isize - l.as_str().len() as isize;

        inner_body = format!("{}{}{}", left, link, right);
    });

    offset = 0;

    let at_pat = Regex::new(r"(@[^,?!+ _(){}]*)")?;
    at_pat
        .captures_iter(inner_body.clone().as_str())
        .for_each(|c| {
            c.iter().skip(1).for_each(|m| {
                if let Some(matched) = m {
                    let left = &inner_body[..(matched.start() as isize + offset) as usize];
                    let right = &inner_body[(matched.end() as isize + offset) as usize..];
                    let text = matched.as_str();

                    let link = format!("<a href=\"{}/{}\">{}</a>", base.url, text, text);
                    offset += link.clone().len() as isize - text.len() as isize;

                    inner_body = format!("{}{}{}", left, link, right);
                }
            });
        });

    Ok(inner_body)
}
