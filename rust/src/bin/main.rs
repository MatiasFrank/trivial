use anyhow::{Error, Result};
use clap::Parser;
use rand::{seq::SliceRandom, thread_rng};
use rust::db::Repository;
use rust::functionality::{self, pause, QuestionID};
use std::fmt::Debug;
// use std::{collections::HashMap, fs};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Database URL
    #[arg(short, long)]
    db: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let url = format!("sqlite://{}", args.db);
    let db = Repository::new(&url).await?;
    let service1 = functionality::Service::new(&db).await?;
    let mut service2 = functionality::QuestionService::new(&db).await?;

    let mut options = service2.get_sets();
    options.sort();
    options.insert(0, String::from("Exit"));
    loop {
        let select = inquire::Select::new("Pick a question set", options.clone());
        let choice = select.prompt()?;
        if choice == "Exit" {
            return Ok(());
        }
        let size = service2.get_set_size(&choice);
        let num = inquire::Text::new(&format!("Number of questions (out of {})", size))
            .with_initial_value(&format!("{}", size))
            .prompt()?
            .parse::<usize>()?;

        let db_questions = service2.get_random_selection(&choice, num);
        let mut questions = service1.get_questions(
            &db_questions
                .iter()
                .map(|q| QuestionID {
                    factory: q.factory.clone(),
                    name: q.name.clone(),
                })
                .collect::<Vec<QuestionID>>(),
        );

        clearscreen::clear()?;
        let mut wrong = Vec::new();
        loop {
            for (i, &q) in questions.iter().enumerate() {
                println!("---------- {}/{} ----------: ", i + 1, questions.len());
                let correct = q.run()?;
                if !correct {
                    wrong.push(q);
                }
                service2.add_answer(&choice, &q.name(), correct).await?;
                // println!("");
            }

            if wrong.is_empty() {
                break;
            }

            let num_correct = questions.len() - wrong.len();
            println!(
                "\n{}/{} correct. Continuing with the remaining {} wrong answers.",
                num_correct,
                questions.len(),
                wrong.len()
            );

            std::mem::swap(&mut wrong, &mut questions);
            wrong.clear();
            questions.shuffle(&mut thread_rng());

            pause()?;
            clearscreen::clear()?;
        }

        pause()?;
        clearscreen::clear()?;
    }
}
