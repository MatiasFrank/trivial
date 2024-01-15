use anyhow::{Error, Result};
use clap::Parser;
use inquire::Text;
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::fs;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the question set
    #[arg(short, long)]
    paths: Vec<String>,

    /// Question start index (inclusive, 1-indexed)
    #[arg(short, long, default_value_t = 1)]
    start: usize,

    /// Question end index (inclusive, 1-indexed)
    #[arg(short, long, default_value_t = 1_000_000)]
    end: usize,
}

#[derive(Serialize, Deserialize, Debug)]
struct BaseQuestionSet {
    name: String,
    type_: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct QuestionSet<T> {
    name: String,
    type_: String,
    data: T,
}

#[derive(Deserialize, Serialize, Debug)]
struct DefaultData {
    items: Vec<DefaultQuestion>,
    question_prefix: String,
}

impl QuestionFactory for DefaultData {
    fn build(&self) -> Result<Vec<Box<dyn Question>>> {
        let mut result = Vec::new();
        for item in &self.items {
            let mut question = item.clone();
            question.question = format!("{}{}?", self.question_prefix, question.question);
            result.push(Box::new(question) as Box<dyn Question>);
        }
        Ok(result)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct DefaultQuestion {
    question: String,
    answers: Vec<String>,
}

impl Question for DefaultQuestion {
    fn run(&self) -> Result<bool> {
        let answer = Text::new(&self.question).prompt()?;
        let correct = self
            .answers
            .iter()
            .any(|a| a.to_lowercase() == answer.to_lowercase());
        if correct {
            println!("Correct!");
        } else {
            println!("Wrong. The answer is {:?}", self.answers[0]);
        }
        Ok(correct)
    }
}

trait Question {
    fn run(&self) -> Result<bool>;
}

trait QuestionFactory {
    fn build(&self) -> Result<Vec<Box<dyn Question>>>;
}

fn main() -> Result<(), Error> {
    let args = Args::parse();

    let mut questions = Vec::new();
    for path in args.paths {
        let contents = fs::read_to_string(path)?;

        let base: BaseQuestionSet = serde_yaml::from_str(&contents)?;

        let factory: Box<dyn QuestionFactory> = match base.type_.as_str() {
            "default" => {
                let set = serde_yaml::from_str::<QuestionSet<DefaultData>>(&contents)?;
                Box::new(set.data) as Box<dyn QuestionFactory>
            }
            _ => {
                panic!("unexpected question type {:?}", base.type_)
            }
        };

        questions.extend(factory.build()?);
    }
    let end = std::cmp::min(args.end, questions.len());
    let mut question_slice: Vec<&Box<dyn Question>> =
        questions[args.start - 1..end].iter().map(|q| q).collect();
    question_slice.shuffle(&mut thread_rng());

    let mut wrong = Vec::new();
    loop {
        for (i, &q) in question_slice.iter().enumerate() {
            print!("{}/{}: ", i + 1, question_slice.len());
            let correct = q.run()?;
            if !correct {
                wrong.push(q);
            }
        }

        if wrong.is_empty() {
            break;
        }

        let num_correct = question_slice.len() - wrong.len();
        println!(
            "\n{}/{} correct. Continuing with the remaining {} wrong answers.",
            num_correct,
            question_slice.len(),
            wrong.len()
        );
        std::mem::swap(&mut wrong, &mut question_slice);
        wrong.clear();
        question_slice.shuffle(&mut thread_rng());
    }

    Ok(())
}
