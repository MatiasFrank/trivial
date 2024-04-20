use anyhow::{bail, Error, Result};
use clap::Parser;
use colored::Colorize;
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Text};
use num_format::{Locale, ToFormattedString};
use rand::{seq::SliceRandom, thread_rng};
use rust::db;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;
use std::{collections::HashMap, fs};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the question set
    #[arg(short, long)]
    path: String,
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
struct NumericRangeData {
    items: Vec<NumericRangeQuestion>,
    question_prefix: String,
    range: f64,
}

impl QuestionFactory for NumericRangeData {
    fn build(
        &self,
        _: &HashMap<String, Box<dyn QuestionFactory>>,
    ) -> Result<Vec<Box<dyn Question>>> {
        let mut result = Vec::new();
        for item in &self.items {
            let mut question = item.clone();
            question.range = self.range;
            question.question = format!("{}{}?", self.question_prefix, question.question);
            result.push(Box::new(question) as Box<dyn Question>);
        }
        Ok(result)
    }
}

fn default_range() -> f64 {
    0.
}

fn si_parse(s: &str) -> Result<i64> {
    let c = if let Some(c) = s.chars().last() {
        c
    } else {
        bail!("empty string");
    };
    if c.is_digit(10) {
        return s.parse::<i64>().map_err(|err| anyhow::Error::from(err));
    }

    let factor: i64 = match c {
        'k' | 'K' => 1_000,
        'm' | 'M' => 1_000_000,
        'g' | 'G' | 'b' | 'B' => 1_000_000_000,
        'T' => 1_000_000_000_000,
        _ => bail!("unexpected last char {}", c),
    };
    let ss = s.get(..s.len() - 1).unwrap();
    let n = (ss.parse::<f64>()? * (factor as f64)) as i64;
    Ok(n)
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct NumericRangeQuestion {
    id: String,
    question: String,
    answer: i64,
    #[serde(default = "default_range")]
    range: f64,
}

impl Question for NumericRangeQuestion {
    fn run(&self) -> Result<bool> {
        let validator = |input: &str| match si_parse(input) {
            Ok(_) => Ok(Validation::Valid),
            Err(err) => Ok(Validation::Invalid(ErrorMessage::Custom(format!(
                "{:?}",
                err
            )))),
        };

        let answer = Text::new(&self.question)
            .with_validator(validator)
            .prompt()?;

        let min = ((self.answer as f64) * (1. - self.range)) as i64;
        let max = ((self.answer as f64) * (1. + self.range)) as i64;
        let a = si_parse(&answer)?;
        let correct = min <= a && a <= max;
        let (min_s, area_s, max_s) = (
            min.to_formatted_string(&Locale::en),
            self.answer.to_formatted_string(&Locale::en),
            max.to_formatted_string(&Locale::en),
        );
        let bound = format!("[{} <= {} <= {}]", min_s, area_s, max_s);
        if correct {
            println!("Within accepted bounds! {}", bound);
        } else {
            println!("Wrong. Accepted bounds: {}", bound);
        }
        println!("");
        Ok(correct)
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct DefaultData {
    items: Vec<DefaultQuestion>,
    question_prefix: String,
}

impl QuestionFactory for DefaultData {
    fn build(
        &self,
        _: &HashMap<String, Box<dyn QuestionFactory>>,
    ) -> Result<Vec<Box<dyn Question>>> {
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
    id: String,
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
        println!("");
        Ok(correct)
    }

    fn id(&self) -> String {
        return self.id.clone();
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct UnionData {
    sets: Vec<String>,
}

struct UnionDataFactory {
    data: UnionData,
}

impl QuestionFactory for UnionDataFactory {
    fn build(
        &self,
        all_factories: &HashMap<String, Box<dyn QuestionFactory>>,
    ) -> Result<Vec<Box<dyn Question>>> {
        let mut res = Vec::new();

        for name in &self.data.sets {
            if let Some(factory) = all_factories.get(name) {
                let questions = factory.build(all_factories)?;
                res.extend(questions);
            } else {
                bail!("unknown question set {}", name);
            }
        }

        Ok(res)
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct VocabData {
    words: Vec<Word>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Word {
    id: String,
    word: String,
    definition: String,
    example: String,
    translations: Vec<String>,
}

struct VocabDataFactory {
    data: VocabData,
}

impl Question for Word {
    fn run(&self) -> Result<bool> {
        let answer = Text::new(&format!("Translation of '{}': ", self.word.bold())).prompt()?;
        let mut correct = true;
        if self.translations.contains(&answer) {
            println!("Valid translation");
        } else {
            correct = false;
            println!("Invalid translation. The accepted ones are:");
            for s in &self.translations {
                println!("\t{}", s);
            }
        }

        pause_with_message("Press any key to see an english definition and example.")?;
        print!("{}", "Definition: ".bold());
        println!("{}", &self.definition);
        print!("{}", "Example: ".bold());
        println!("{}", &self.example);

        let ans = Confirm::new("Did you know the definition?").prompt()?;
        Ok(correct && ans)
    }

    fn id(&self) -> String {
        self.id.clone()
    }
}

impl QuestionFactory for VocabDataFactory {
    fn build(
        &self,
        _: &HashMap<String, Box<dyn QuestionFactory>>,
    ) -> Result<Vec<Box<dyn Question>>> {
        let mut res = Vec::new();

        for word in &self.data.words {
            res.push(Box::new(word.clone()) as Box<dyn Question>);
        }

        Ok(res)
    }
}

trait Question {
    fn run(&self) -> Result<bool>;
    fn id(&self) -> String;
}

trait QuestionFactory {
    fn build(
        &self,
        all_factories: &HashMap<String, Box<dyn QuestionFactory>>,
    ) -> Result<Vec<Box<dyn Question>>>;
}

fn pause_with_message(msg: &str) -> Result<()> {
    let mut stdout = stdout();
    stdout.write(msg.as_bytes())?;
    stdout.flush().unwrap();
    stdin().read(&mut [0])?;
    Ok(())
}

fn pause() -> Result<()> {
    pause_with_message("Press any key to continue...")
}

fn load_factories(paths: &[PathBuf]) -> Result<HashMap<String, Box<dyn QuestionFactory>>> {
    let mut factories = HashMap::new();
    for path in paths {
        let contents = fs::read_to_string(path)?;
        let base: BaseQuestionSet = serde_yaml::from_str(&contents)?;
        let (factory, name) = match base.type_.as_str() {
            "default" => {
                let set = serde_yaml::from_str::<QuestionSet<DefaultData>>(&contents)?;
                (Box::new(set.data) as Box<dyn QuestionFactory>, set.name)
            }
            "numeric_range" => {
                let set = serde_yaml::from_str::<QuestionSet<NumericRangeData>>(&contents)?;
                (Box::new(set.data) as Box<dyn QuestionFactory>, set.name)
            }
            "union" => {
                let set = serde_yaml::from_str::<QuestionSet<UnionData>>(&contents)?;
                (
                    Box::new(UnionDataFactory { data: set.data }) as Box<dyn QuestionFactory>,
                    set.name,
                )
            }
            "vocab" => {
                let set = serde_yaml::from_str::<QuestionSet<VocabData>>(&contents)?;
                (
                    Box::new(VocabDataFactory { data: set.data }) as Box<dyn QuestionFactory>,
                    set.name,
                )
            }
            _ => {
                panic!("unexpected question type {:?}", base.type_)
            }
        };
        factories.insert(name, factory);
    }

    Ok(factories)
}

fn quiz_loop(factories: &HashMap<String, Box<dyn QuestionFactory>>) -> Result<()> {
    loop {
        let mut options: Vec<String> = factories.keys().map(|s| s.clone()).collect();
        options.sort();
        options.insert(0, String::from("Exit"));
        let select = inquire::Select::new("Pick a question set", options);
        let choice = select.prompt()?;
        if choice == "Exit" {
            return Ok(());
        }
        let factory = factories.get(&choice).unwrap();
        let questions = factory.build(&factories)?;

        let start = inquire::Text::new("Start index:")
            .with_initial_value("1")
            .prompt()?
            .parse::<usize>()?;
        let end = inquire::Text::new("End index:")
            .with_initial_value(&format!("{}", questions.len()))
            .prompt()?
            .parse::<usize>()?;
        let mut question_slice: Vec<&Box<dyn Question>> =
            questions[start - 1..end].iter().map(|q| q).collect();
        question_slice.shuffle(&mut thread_rng());

        clearscreen::clear()?;
        let mut wrong = Vec::new();
        loop {
            for (i, &q) in question_slice.iter().enumerate() {
                println!("---------- {}/{} ----------: ", i + 1, question_slice.len());
                let correct = q.run()?;
                if !correct {
                    wrong.push(q);
                }
                println!("");
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

            pause()?;
            clearscreen::clear()?;
        }

        pause()?;
        clearscreen::clear()?;
    }
}

fn main() -> Result<(), Error> {
    let args = Args::parse();
    let mut paths = Vec::new();
    for path in fs::read_dir(args.path)? {
        paths.push(path?.path());
    }
    let factories = load_factories(&paths)?;
    quiz_loop(&factories)
}
