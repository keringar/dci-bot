use chrono::prelude::*;
use failure::Error;
use orca;
use rusqlite::Connection;
use scraper::EventListing;
use serde_json;
use std;
use strings::*;
use url::form_urlencoded::byte_serialize;

pub struct DCIBot {
    connection: Connection,
}

impl DCIBot {
    pub fn new() -> Result<DCIBot, Error> {
        let connection = Connection::open(DB_PATH)?;

        Ok(DCIBot { connection })
    }

    pub fn run_forever(&self) {
        if let Err(e) = self.actual_run_forever() {
            eprintln!("Runtime error in bot thread: {}", e);
            std::process::exit(-1);
        }
    }

    fn actual_run_forever(&self) -> Result<(), Error> {
        use chrono;
        use std::{thread, time};

        loop {
            let now = Utc::now();

            // Get events within the next 24 hours
            let matching_events = self.get_events_matching(now)?;

            // If the time until the closest event is less than 10 hours away, post next n
            // events in the next 10-(10 + 4 * n) evvents
            let mut posted_events = Vec::new();
            let mut time_to_search = 10;
            for event in matching_events {
                let time_since_today = event.event_date.signed_duration_since(now);

                if time_since_today <= chrono::Duration::hours(time_to_search) {
                    time_to_search += 4;
                    posted_events.push(event);
                }
            }

            self.create_post(&posted_events)?;

            // Sleep an hour
            thread::sleep(time::Duration::from_secs(60 * 60));
        }
    }

    // Gets all events within the next 24 hours of the specified date
    fn get_events_matching(&self, date: DateTime<Utc>) -> Result<Vec<EventListing>, Error> {
        use chrono;

        let today = date;
        let tomorrow = date + chrono::Duration::days(1);

        let sql_today = format!(
            r"SELECT * FROM events WHERE date LIKE '{}-{:02}-{:02}%' AND posted IS NULL",
            today.year(),
            today.month(),
            today.day()
        );

        let sql_tomorrow = format!(
            r"SELECT * FROM events WHERE date LIKE '{}-{:02}-{:02}%' AND posted IS NULL",
            tomorrow.year(),
            tomorrow.month(),
            tomorrow.day()
        );

        let mut today_stmt = self.connection.prepare(&sql_today)?;
        let today_rows = today_stmt.query_and_then(&[], |row| {
            self.create_listing_from_strings(
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
                row.get(5),
                row.get(6),
            )
        })?;

        let mut tomorrow_stmt = self.connection.prepare(&sql_tomorrow)?;
        let tomorrow_rows = tomorrow_stmt.query_and_then(&[], |row| {
            self.create_listing_from_strings(
                row.get(1),
                row.get(2),
                row.get(3),
                row.get(4),
                row.get(5),
                row.get(6),
            )
        })?;

        let listings: Result<Vec<EventListing>, Error> = today_rows.chain(tomorrow_rows).collect();

        match listings {
            Ok(listings) => {
                let filtered_listings = listings
                    .into_iter()
                    .filter(|event| {
                        let time_since_today = event.event_date.signed_duration_since(today);

                        println!(
                            "Event {} is {} hours and {} minutes away",
                            event.title,
                            time_since_today.num_hours(),
                            time_since_today.num_minutes() - time_since_today.num_hours() * 60
                        );
                        time_since_today <= chrono::Duration::days(1)
                            && time_since_today >= chrono::Duration::zero()
                    })
                    .collect();

                Ok(filtered_listings)
            }
            Err(e) => Err(e),
        }
    }

    fn create_listing_from_strings(
        &self,
        url: String,
        date: String,
        location: String,
        title: String,
        timezone: String,
        lineup: String,
    ) -> Result<EventListing, Error> {
        let date = DateTime::parse_from_rfc3339(&date)?;
        let events: Vec<(String, String)> = serde_json::from_str(&lineup)?;

        Ok(EventListing {
            event_url: url,
            event_date: date,
            location,
            title,
            timezone,
            lineup: events,
        })
    }

    fn create_post(&self, events: &Vec<EventListing>) -> Result<(), Error> {
        // Update DB to show we've already posted
        let now = Utc::now();
        for event in events {
            println!("Posting {}", event.title);

            self.connection.execute(
                "UPDATE events SET posted=? WHERE date=?",
                &[&now, &event.event_date],
            )?;
        }

        // Generate the table and stuff
        let mut title = if let Some(event) = events.iter().next() {
            format!(
                "[Show Thread] {}/{}:",
                event.event_date.month(),
                event.event_date.day()
            )
        } else {
            return Ok(());
        };

        for event in events {
            let string = format!(" {} - {} |", event.title, event.location);
            title.push_str(&string);
        }
        title.pop();

        let mut text = String::new();
        for event in events {
            let string = format!(
                "**{} - {}**\n\n[DCI Page]({})\n\n**Lineup & Times**\n\n*All times {} and subject to change*\n\n",
                event.title, event.location, event.event_url, event.timezone
            );
            text.push_str(&string);

            for (iteration, lineup_event) in event.lineup.iter().enumerate() {
                if iteration == 0 {
                    let first = format!("| {} | {} |\n", lineup_event.0, lineup_event.1);
                    let next = format!("|------|-----------------------------------------|\n");

                    text.push_str(&first);
                    text.push_str(&next);
                } else {
                    let string = format!(" {} | {}\n", lineup_event.0, lineup_event.1);
                    text.push_str(&string);
                }
            }

            if events.len() >= 2 {
                text.push_str("\n---\n\n");
            }
        }

        let url_encoded_text: String = byte_serialize(text.as_bytes()).collect();

        // Look for reddit login info
        use std::env::var;
        let (pass, secret) = match (var("DCI_PASSWORD"), var("DCI_SECRET")) {
            (Ok(pass), Ok(secret)) => (pass, secret),
            (_, _) => bail!("DCI_PASSWORD and/or DCI_SECRET not set"),
        };

        let mut reddit = orca::App::new("/r/drumcorps show bot", "0.1", "warren")?;
        reddit.authorize_script("AOBwXdKkVWSjTg", &secret, "DrumCorpsBot", &pass)?;
        reddit.submit_self("/r/Gumland", &title, &url_encoded_text, false)?;

        Ok(())
    }
}
