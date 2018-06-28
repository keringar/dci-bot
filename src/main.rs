extern crate chrono;
#[macro_use]
extern crate failure;
extern crate reqwest;
extern crate rusqlite;
extern crate select;

mod scraper;
mod strings;

fn main() {
    println!("Starting DCI automated notification bot");

    let scraper = match scraper::DCIScraper::new() {
        Ok(scraper) => scraper,
        Err(e) => {
            eprintln!("Couldn't start scraper: {}", e);
            std::process::exit(-1);
        }
    };

    if let Err(e) = scraper.loop_forever() {
        eprintln!("Runtime error: {}", e);
        std::process::exit(-1);
    }
}
