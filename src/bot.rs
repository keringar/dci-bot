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
            let now = Local::now();

            // Get events within the next 24 hours
            let mut matching_events =
                self.get_events_matching(now.with_timezone(&chrono::offset::Utc))?;

            if matching_events.len() > 0 {
                matching_events.sort_unstable_by(|a, b| a.event_date.cmp(&b.event_date));

                if let Some(event) = matching_events.iter().next() {
                    let elapsed_time = event.event_date.signed_duration_since(now);

                    println!(
                        "{} hours until {} starts",
                        elapsed_time.num_hours(),
                        event.title
                    );

                    if elapsed_time.num_hours() < 12 {
                        self.create_post(&matching_events)?;
                    }
                }
            }

            // Sleep an hour
            thread::sleep(time::Duration::from_secs(60 * 60));
        }
    }

    // Gets all events within the next 24 hours of the specified date
    fn get_events_matching(&self, date: DateTime<Utc>) -> Result<Vec<EventListing>, Error> {
        let sql_today = format!(
            r"SELECT * FROM events WHERE human_date LIKE '{}/{}%' AND posted IS NULL",
            date.month(),
            date.day()
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
                row.get(8),
            )
        })?;

        let found: Result<Vec<EventListing>, Error> = today_rows.collect();

        if let Ok(ref found) = found {
            println!(
                "Found {} DCI events on {}/{}",
                found.len(),
                date.month(),
                date.day(),
            );
        }

        found
    }

    fn create_listing_from_strings(
        &self,
        url: String,
        date: String,
        location: String,
        title: String,
        timezone: String,
        lineup: String,
        human_date: String,
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
            human_date,
        })
    }

    fn create_post(&self, events: &Vec<EventListing>) -> Result<(), Error> {
        // Generate the table and stuff
        let mut title = if let Some(event) = events.iter().next() {
            format!("[Show Thread] {}:", event.human_date)
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
        let (pass, secret) = match (var(ENV_PASSWORD), var(ENV_SECRET)) {
            (Ok(pass), Ok(secret)) => (pass, secret),
            (_, _) => bail!("DCI_PASSWORD and/or DCI_SECRET not set"),
        };

        let mut reddit = orca::App::new("/r/drumcorps show bot", "0.1", "warren")?;
        reddit.authorize_script(APP_ID, &secret, USERNAME, &pass)?;
        reddit.submit_self(SUBREDDIT, &title, &url_encoded_text, false)?;

        // Update DB to show we've already posted
        let now = Utc::now();
        for event in events {
            println!("Posting {}", event.title);

            self.connection.execute(
                "UPDATE events SET posted=? WHERE date=?",
                &[&now, &event.event_date],
            )?;
        }

        Ok(())
    }
}
