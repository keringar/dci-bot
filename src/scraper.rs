use chrono;
use chrono::prelude::*;
use failure::Error;
use reqwest;
use rusqlite::Connection;
use select::{document::Document, predicate::*};
use strings::*;

/// Event listing that is stored in the database
#[derive(Debug)]
struct EventListing {
    /// Event date in a provided timezone
    event_date: DateTime<FixedOffset>,
    /// City and state string. i.e. "Madson, OH"
    location: String,
    /// Name of the event. i.e. "Summer Music Games in Cincinnati"
    title: String,
    /// Human readable timezone (pulled from DCI, not the found time)
    timezone: String,
    /// Sorted lineup that contains a time and an associated event
    lineup: Vec<(String, String)>,
}

#[derive(Debug)]
pub struct DCIScraper {
    connection: Connection,
}

impl DCIScraper {
    pub fn new() -> Result<DCIScraper, Error> {
        let connection = Connection::open(DB_PATH)?;

        Ok(DCIScraper { connection })
    }

    pub fn loop_forever(self) -> Result<(), Error> {
        use std::{thread, time};

        loop {
            // If we found something, update the db
            for entry in self.scrape(Utc::now() + chrono::Duration::days(1))? {
                self.write_to_db(&entry);
            }

            // Sleep an hour
            thread::sleep(time::Duration::from_secs(60 * 60));
        }
    }

    // Scrape tomorrow's event list
    fn scrape(&self, date: DateTime<Utc>) -> Result<Vec<EventListing>, Error> {
        // Parse the webpage
        let response = self.get_event_page_at(&date)?;
        let document = Document::from_read(response)?;

        // Scrape the lineup and timezone from the details webpage
        let mut results = Vec::new();
        if let Some(container) = document.find(Class(ITEMS_PARENT_CONTAINER)).next() {
            // Each item is an event
            for child in container.children() {
                // Find the details link to parse the timezone and linup
                let details_url: String = child
                    .find(Attr("class", ITEMS_LINK_DETAILS))
                    .filter_map(|n| n.attr("href"))
                    .take(1)
                    .collect();
                let (lineup, timezone) = self.scrape_details(&details_url)?;

                // Get the info section of the event box
                let info_box = child.find(Attr("class", INFO_SECTION_CLASS)).next();
                let (title, event_date, location) = match info_box {
                    Some(info) => {
                        // Parse title
                        let title = match info.find(Name("h3")).next() {
                            Some(title) => title.text(),
                            None => bail!("Couldn't parse event title. Did the website change?"),
                        };

                        // Parse event date
                        let date_marker = Attr("src", INFO_DATE_MARKER);
                        let date = match info.find(date_marker).next() {
                            Some(date) => match date.attr("alt") {
                                Some(ts) => {
                                    DateTime::parse_from_str(ts, "%Y-%m-%dT%H:%M:%S%.3f%z")?
                                }
                                None => {
                                    bail!("Couldn't get date timestamp. Did the website change?")
                                }
                            },
                            None => bail!("Couldn't parse date. Did the website change?"),
                        };

                        // Parse location
                        let location_marker = Attr("src", INFO_LOCATION_MARKER);
                        let location = match info.find(location_marker).next() {
                            Some(location) => match location.parent() {
                                Some(parent) => parent.text().trim().to_string(),
                                None => bail!("Couldn't find location text"),
                            },
                            None => bail!("Couldn't find location marker. Did the website change?"),
                        };

                        (title, date, location)
                    }
                    None => bail!("Couldn't find info box"),
                };

                let listing = EventListing {
                    event_date,
                    location,
                    title,
                    timezone,
                    lineup,
                };

                results.push(listing);
            }
        }

        Ok(results)
    }

    fn scrape_details(&self, url: &str) -> Result<(Vec<(String, String)>, String), Error> {
        // Load the details page
        let event_page_url = format!("{}{}", BASE_URL, url);
        let response = reqwest::get(&event_page_url)?;

        println!("Found event at {}, scraping", event_page_url);

        // Parse the HTML
        let document = Document::from_read(response)?;

        // Parse timezone
        let timezone_predicate = Attr("class", TZ_CONTENT_CONTAINER).descendant(Name("p"));
        let timezone = match document.find(timezone_predicate).next() {
            Some(node) => match node.children().map(|n| n.text()).nth(1) {
                Some(tz) => tz,
                None => bail!("Couldn't parse timezone. Did the website change?"),
            },
            None => bail!("Couldn't parse timezone section. Did the website change?"),
        };

        // Parse the table
        let lineup = match document.find(Attr("class", TIME_TABLE)).next() {
            Some(node) => {
                let mut lineup = Vec::new();
                for child in node.children() {
                    let (time, event) = match (child.first_child(), child.last_child()) {
                        (Some(first), Some(last)) => (first.text(), last.text()),
                        (_, _) => bail!("Couldn't parse lineup. Did the website change?"),
                    };

                    lineup.push((time, event));
                }

                lineup
            }
            None => bail!("Couldn't parse times table. Did the website change?"),
        };

        Ok((lineup, timezone))
    }

    fn write_to_db(&self, event: &EventListing) {
        println!("{:?}", event);
    }

    fn get_event_page_at(&self, date: &DateTime<Utc>) -> Result<reqwest::Response, reqwest::Error> {
        let date_query = format!("{}-{:02}-{:02}", date.year(), date.month(), date.day());

        let client = reqwest::Client::new();
        client
            .get(EVENT_URL)
            .query(&[
                (START_TIME_QUERY, &date_query),
                (END_TIME_QUERY, &date_query),
            ])
            .send()
    }
}
