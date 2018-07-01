extern crate chrono;
#[macro_use]
extern crate failure;
extern crate reqwest;
extern crate rusqlite;
extern crate select;
extern crate serde;
extern crate serde_json;

mod bot;
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

    if let Err(e) = bot_thread.join().expect("Bot thread panicked!") {
        eprintln!("Runtime error in bot thread: {}", e);
        std::process::exit(-1);
    }
}
