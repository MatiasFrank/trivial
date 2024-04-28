use anyhow::{Error, Result};
use chrono::Utc;
use clap::Parser;
use core::fmt;
use rand::{seq::SliceRandom, thread_rng};
use rust::db::Repository;
use rust::functionality::{self, pause, Selection, Service};
use std::fmt::Debug;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Database URL
    #[arg(short, long)]
    db: String,
}

#[derive(Clone, PartialEq, Eq)]
enum Choice {
    Value(String),
    Exit,
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Choice::Value(s) => {
                write!(f, "{}", s)
            }
            Choice::Exit => {
                write!(f, "Exit")
            }
        }
    }
}

#[derive(Clone)]
enum Method {
    Bottom,
    WeightedRandom,
    UniformRandom,
    OldestAnswer,
}
impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Method::Bottom => write!(f, "Bottom"),
            Method::WeightedRandom => write!(f, "Weighted random"),
            Method::UniformRandom => write!(f, "Uniform random"),
            Method::OldestAnswer => write!(f, "Oldest answer"),
        }
    }
}

#[derive(Clone)]
struct Choice2 {
    choice: Choice,
    method: Method,
    selection: Selection,
    num: usize,
}

fn get_choice(service: &Service, last_choice: &Option<Choice2>) -> Result<Choice2> {
    if let Some(choice) = last_choice {
        if inquire::Confirm::new("Start again with same choice?").prompt()? {
            return Ok(choice.clone());
        }
    }

    let mut options = vec![Choice::Exit];
    for s in service.get_sets() {
        options.push(Choice::Value(s.clone()));
    }
    let select = inquire::Select::new("Pick a question set", options);
    let choice = match select.prompt()? {
        Choice::Value(s) => s,
        Choice::Exit => {
            return Ok(Choice2 {
                choice: Choice::Exit,
                method: Method::Bottom,
                selection: Selection::All,
                num: 0,
            })
        }
    };
    let selection = inquire::Select::new(
        "Selection method",
        vec![Selection::All, Selection::Practiced],
    )
    .prompt()?;
    let size = service.get_set_size(&choice, selection);
    let num = inquire::Text::new(&format!("Number of questions (out of {})", size))
        .with_initial_value(&format!("{}", size))
        .prompt()?
        .parse::<usize>()?;
    let method = inquire::Select::new(
        "Ranking method",
        vec![
            Method::Bottom,
            Method::WeightedRandom,
            Method::UniformRandom,
            Method::OldestAnswer,
        ],
    )
    .prompt()?;

    Ok(Choice2 {
        choice: Choice::Value(choice),
        method,
        selection,
        num,
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Args::parse();
    let url = format!("sqlite://{}", args.db);
    let db = Repository::new(&url).await?;
    let now = Instant::now();
    let mut service = functionality::Service::new(&db).await?;
    println!("Time to load: {:?}", now.elapsed());
    let mut last_choice: Option<Choice2> = None;
    loop {
        let choice = get_choice(&service, &last_choice)?;
        let set = if let Choice::Value(set) = &choice.choice {
            set
        } else {
            return Ok(());
        };

        let mut question_ids = match choice.method {
            Method::Bottom => service.get_bottom_selection(&set, choice.num, choice.selection),
            Method::WeightedRandom => {
                service.get_weighted_random_selection(&set, choice.num, choice.selection)
            }
            Method::UniformRandom => {
                service.get_uniform_random_selection(&set, choice.num, choice.selection)
            }
            Method::OldestAnswer => service.get_oldest_answer(&set, choice.num, choice.selection),
        };
        clearscreen::clear()?;
        let mut wrong = Vec::new();
        loop {
            question_ids.shuffle(&mut thread_rng());
            for (i, &id) in question_ids.iter().enumerate() {
                println!("---------- {}/{} ----------: ", i + 1, question_ids.len());
                let since_str = if let Some(answer) = service.last_answer(id) {
                    let since = Utc::now().signed_duration_since(answer.time);
                    format!("{:?}", since.to_std()?)
                } else {
                    String::from("-")
                };
                let question = service.get(id);
                println!(
                    "prob: {:.3}, last answered: {}",
                    question.probability, since_str
                );
                let correct = question.runner.run()?;
                if !correct {
                    wrong.push(id);
                }
                service.add_answer(id, correct).await?;
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
        last_choice = Some(choice);
    }
}
