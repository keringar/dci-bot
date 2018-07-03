extern crate chrono;
#[macro_use]
extern crate failure;
extern crate orca;
extern crate reqwest;
extern crate rusqlite;
extern crate select;
extern crate serde;
extern crate serde_json;
extern crate url;

mod bot;
mod scraper;
mod strings;

fn main() {
    if std::env::var("DCI_PASSWORD").is_err() || std::env::var("DCI_SECRET").is_err() {
        eprintln!("DCI_PASSWORD and/or DCI_SECRET not set");
        std::process::exit(-1);
    }

    println!("Starting DCI automated notification bot");

    let scraper = match scraper::DCIScraper::new() {
        Ok(scraper) => scraper,
        Err(e) => {
            eprintln!("Couldn't start scraper: {}", e);
            std::process::exit(-1);
        }
    };

    let bot = match bot::DCIBot::new() {
        Ok(bot) => bot,
        Err(e) => {
            eprintln!("Couldn't start notification bot: {}", e);
            std::process::exit(-1);
        }
    };

    let bot_thread = std::thread::spawn(move || bot.run_forever());

    if let Err(e) = scraper.loop_forever() {
        eprintln!("Runtime error: {}", e);
        std::process::exit(-1);
    }

    bot_thread.join().expect("Bot thread panicked!");
}
