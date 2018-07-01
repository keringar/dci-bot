use chrono::prelude::*;
use failure::Error;
use rusqlite::Connection;
use scraper::EventListing;
use serde_json;
use strings::*;

pub struct DCIBot {
    connection: Connection,
}

impl DCIBot {
    pub fn new() -> Result<DCIBot, Error> {
        let connection = Connection::open(DB_PATH)?;

        Ok(DCIBot { connection })
    }

    pub fn run_forever(&self) -> Result<(), Error> {
        use std::{thread, time};

        loop {
            for event in self.get_events_matching(Utc::now())? {
                self.create_post(&event)?;
            }

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
            r"SELECT * FROM events WHERE date LIKE '{}-{:02}-{:02}%'",
            today.year(),
            today.month(),
            today.day()
        );

        let sql_tomorrow = format!(
            r"SELECT * FROM events WHERE date LIKE '{}-{:02}-{:02}%'",
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

    fn create_post(&self, event: &EventListing) -> Result<(), Error> {
        println!("{:?}", event);

        Ok(())
    }
}
