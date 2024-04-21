use anyhow::{Error, Result};
use clap::Parser;
use rand::{seq::SliceRandom, thread_rng};
use rust::db::Repository;
use rust::functionality::{self, pause, Service};
use std::ascii::AsciiExt;
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Database URL
    #[arg(short, long)]
    db: String,
}

fn get_choice(
    service: &Service,
    last_choice: &Option<(String, usize, String)>,
) -> Result<(String, usize, String, bool)> {
    if let Some((choice, num, method)) = last_choice {
        if inquire::Confirm::new("Start again with same choice?").prompt()? {
            return Ok((choice.clone(), *num, method.clone(), true));
        }
    }

    let mut options = service.get_sets();
    options.sort();
    options.insert(0, String::from("Exit"));

    let select = inquire::Select::new("Pick a question set", options.clone());
    let choice = select.prompt()?;
    if choice == "Exit" {
        return Ok((String::new(), 0, String::new(), false));
    }
    let size = service.get_set_size(&choice);
    let num = inquire::Text::new(&format!("Number of questions (out of {})", size))
        .with_initial_value(&format!("{}", size))
        .prompt()?
        .parse::<usize>()?;
    let method =
        inquire::Select::new("Selection method", vec!["Bottom", "Weighted random"]).prompt()?;

    Ok((choice, num, String::from_str(method)?, true))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let url = format!("sqlite://{}", args.db);
    let db = Repository::new(&url).await?;
    let mut service = functionality::Service::new(&db).await?;
    let mut last_choice: Option<(String, usize, String)> = None;
    loop {
        let (choice, num, method, cont) = get_choice(&service, &last_choice)?;
        if !cont {
            return Ok(());
        }

        let mut question_ids = match method.as_str() {
            "Weighted random" => service.get_random_selection(&choice, num),
            "Bottom" => service.get_bottom_selection(&choice, num),
            _ => panic!("bad choice {}", method),
        };
        clearscreen::clear()?;
        let mut wrong = Vec::new();
        loop {
            question_ids.shuffle(&mut thread_rng());
            for (i, id) in question_ids.iter().enumerate() {
                println!("---------- {}/{} ----------: ", i + 1, question_ids.len());
                println!("prob: {:.3}", service.get(id).probability);
                let correct = service.get(id).runner.run()?;
                if !correct {
                    wrong.push(id.clone());
                }
                service.add_answer(id.clone(), correct).await?;
            }

            if wrong.is_empty() {
                break;
            }

            let num_correct = question_ids.len() - wrong.len();
            println!(
                "\n{}/{} correct. Continuing with the remaining {} wrong answers.",
                num_correct,
                question_ids.len(),
                wrong.len()
            );

            std::mem::swap(&mut wrong, &mut question_ids);
            wrong.clear();

            pause()?;
            clearscreen::clear()?;
        }
        pause()?;
        clearscreen::clear()?;
        last_choice = Some((choice, num, method));
    }
}
