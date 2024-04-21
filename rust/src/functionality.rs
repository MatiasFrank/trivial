use crate::db;
use anyhow::{bail, Result};
use colored::Colorize;
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Text};
use num_format::{Locale, ToFormattedString};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs;
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseQuestionSet {
    name: String,
    type_: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QuestionFactoryModel<T1: Question, T2> {
    name: String,
    type_: String,
    items: Vec<T1>,
    data: T2,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct NumericRangeData {
    question_prefix: String,
    range: f64,
}

impl QuestionFactory for NumericRangeData {
    fn build(&self, data: &[u8]) -> Result<Box<dyn Question>> {
        let mut question = serde_yaml::from_slice::<NumericRangeQuestion>(data)?;
        question.range = self.range;
        question.question = format!("{}{}?", self.question_prefix, question.question);
        Ok(Box::new(question) as Box<dyn Question>)
    }

    // fn name(&self) -> String {
    //     self.name.clone()
    // }
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

    fn name(&self) -> String {
        self.id.clone()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct DefaultData {
    question_prefix: String,
}

impl QuestionFactory for DefaultData {
    fn build(&self, data: &[u8]) -> Result<Box<dyn Question>> {
        let mut question = serde_yaml::from_slice::<DefaultQuestion>(data)?;
        question.question = format!("{}{}?", self.question_prefix, question.question);
        Ok(Box::new(question) as Box<dyn Question>)
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

    fn name(&self) -> String {
        return self.id.clone();
    }
}

#[derive(Deserialize, Serialize, Debug)]
struct UnionData {
    sets: Vec<String>,
}

// struct UnionDataFactory {
//     name: String,
//     data: UnionData,
// }

// impl QuestionFactory for UnionDataFactory {
//     fn build(
//         &self,
//         all_factories: &HashMap<String, Box<dyn QuestionFactory>>,
//     ) -> Result<Vec<Box<dyn Question>>> {
//         let mut res = Vec::new();

//         for name in &self.data.sets {
//             if let Some(factory) = all_factories.get(name) {
//                 let questions = factory.build(all_factories)?;
//                 res.extend(questions);
//             } else {
//                 bail!("unknown question set {}", name);
//             }
//         }

//         Ok(res)
//     }

//     // fn depends_on(&self) -> Vec<String> {
//     //     self.data.sets.clone()
//     // }

//     fn name(&self) -> String {
//         self.name.clone()
//     }
// }

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Word {
    id: String,
    word: String,
    definition: String,
    example: String,
    translations: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct VocabData {}

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

    fn name(&self) -> String {
        self.id.clone()
    }
}

pub fn pause() -> Result<()> {
    pause_with_message("Press any key to continue...")
}

impl QuestionFactory for VocabData {
    fn build(&self, data: &[u8]) -> Result<Box<dyn Question>> {
        let question = serde_yaml::from_slice::<Word>(data)?;
        Ok(Box::new(question) as Box<dyn Question>)
    }
}

pub trait Question {
    fn run(&self) -> Result<bool>;
    fn name(&self) -> String;
    // fn set(&self) -> String;
}

pub trait QuestionFactory {
    fn build(&self, data: &[u8]) -> Result<Box<dyn Question>>;
}

fn pause_with_message(msg: &str) -> Result<()> {
    let mut stdout = stdout();
    stdout.write(msg.as_bytes())?;
    stdout.flush().unwrap();
    stdin().read(&mut [0])?;
    Ok(())
}

pub struct Service {
    question_sets: HashMap<String, HashMap<String, Box<dyn Question>>>,
}

pub struct QuestionID {
    pub factory: String,
    pub name: String,
}

impl Service {
    pub async fn new(db: &db::Repository) -> Result<Service> {
        let questions = db.get_all_questions().await?;
        let factories = load_factories(&db.get_all_question_factories().await?)?;

        let mut question_impls = HashMap::new();
        for q in questions {
            if !question_impls.contains_key(&q.factory) {
                question_impls.insert(q.factory.clone(), HashMap::new());
            }
            let set = question_impls.get_mut(&q.factory).unwrap();
            let factory = factories.get(&q.factory).unwrap();
            set.insert(q.name, factory.build(&q.data)?);
        }

        Ok(Service {
            question_sets: question_impls,
        })
    }

    pub fn get_questions(&self, ids: &[QuestionID]) -> Vec<&Box<dyn Question>> {
        ids.iter()
            .map(|id| {
                self.question_sets
                    .get(&id.factory)
                    .unwrap()
                    .get(&id.name)
                    .unwrap()
            })
            .collect::<Vec<&Box<dyn Question>>>()
        // Vec::new()
    }
}

pub fn load_factories(
    factory_models: &Vec<db::QuestionFactory>,
) -> Result<HashMap<String, Box<dyn QuestionFactory>>> {
    let mut factories = HashMap::new();
    for f in factory_models {
        let factory = match f.factory_type.as_str() {
            "default" => {
                let f = serde_yaml::from_slice::<DefaultData>(&f.data)?;
                Box::new(f) as Box<dyn QuestionFactory>
            }
            "numeric_range" => {
                let f = serde_yaml::from_slice::<NumericRangeData>(&f.data)?;
                Box::new(f) as Box<dyn QuestionFactory>
            }
            "vocab" => {
                let f = serde_yaml::from_slice::<VocabData>(&f.data)?;
                Box::new(f) as Box<dyn QuestionFactory>
            }
            "union" => {
                continue;
            }
            _ => {
                panic!("unexpected question type {:?}", f.factory_type);
            }
        };
        factories.insert(f.name.clone(), factory);
    }

    Ok(factories)
}

pub struct Models {
    pub questions: Vec<db::Question>,
    pub factories: Vec<db::QuestionFactory>,
    // sets: HashMap<String, Vec<QuestionID>>,
}

fn parse_factory<T1, T2>(models: &mut Models, stuff: &QuestionFactoryModel<T1, T2>) -> Result<()>
where
    T1: Serialize + Question,
    T2: Serialize,
{
    // let stuf = serde_yaml::from_slice::<QuestionFactoryModel<T1, T2>>(data)?;
    for q in &stuff.items {
        let data = serde_yaml::to_vec(&q)?;
        models.questions.push(db::Question {
            factory: stuff.name.clone(),
            name: q.name(),
            data,
            ..Default::default()
        });
    }

    models.factories.push(db::QuestionFactory {
        id: 0,
        name: stuff.name.clone(),
        factory_type: stuff.type_.clone(),
        data: serde_yaml::to_vec(&stuff.data)?,
    });
    Ok(())
}

// fn parse_set<'de, T1, T2>(models: &mut Models, data: &'de [u8]) -> Result<()>
// where
//     T1: Serialize + Deserialize<'de> + Question,
//     T2: Serialize + Deserialize<'de>,
// {
//     let stuff = serde_yaml::from_slice::<QuestionFactoryModel<T1, T2>>(data)?;
//     for q in stuff.items {
//         let data = serde_yaml::to_vec(&q)?;
//         models.questions.push(db::Question {
//             factory: stuff.name.clone(),
//             name: q.name(),
//             data,
//             ..Default::default()
//         });
//     }
//     Ok(())
// }

pub fn load_models(paths: &[PathBuf]) -> Result<Models> {
    let mut models = Models {
        questions: Vec::new(),
        factories: Vec::new(),
        // sets: HashMap::new(),
    };
    for p in paths {
        println!("path: {:?}", p);
        let data = fs::read(p)?;
        let set = serde_yaml::from_slice::<BaseQuestionSet>(&data)?;
        match set.type_.as_str() {
            "default" => {
                let stuff = serde_yaml::from_slice::<
                    QuestionFactoryModel<DefaultQuestion, DefaultData>,
                >(&data)?;
                parse_factory::<DefaultQuestion, DefaultData>(&mut models, &stuff)?;
            }
            "numeric_range" => {
                let stuff = serde_yaml::from_slice::<
                    QuestionFactoryModel<NumericRangeQuestion, NumericRangeData>,
                >(&data)?;
                parse_factory::<NumericRangeQuestion, NumericRangeData>(&mut models, &stuff)?;
            }
            "vocab" => {
                let stuff = serde_yaml::from_slice::<QuestionFactoryModel<Word, VocabData>>(&data)?;
                parse_factory::<Word, VocabData>(&mut models, &stuff)?;
            }
            "union" => {}
            _ => {
                panic!("unexpected question type {:?}", set.type_);
            }
        };
    }

    Ok(models)
}

struct ProbQuestion {
    answers: Vec<db::Answer>,
    probability: f64,
}

struct ProbabilityComputer {
    questions: HashMap<i64, ProbQuestion>,
}

impl ProbabilityComputer {
    fn new(answers: &Vec<db::Answer>, questions: &Vec<db::Question>) -> ProbabilityComputer {
        let mut questions2 = HashMap::new();
        for q in questions {
            questions2.insert(
                q.id,
                ProbQuestion {
                    answers: Vec::new(),
                    probability: q.probability,
                },
            );
        }

        for a in answers {
            questions2
                .get_mut(&a.question_id)
                .unwrap()
                .answers
                .push(a.clone());
        }

        for (_, q) in questions2.iter_mut() {
            q.answers.sort_by_key(|a| a.time);
        }

        ProbabilityComputer {
            questions: questions2,
        }
    }

    fn add_answer(&mut self, answer: db::Answer) -> f64 {
        let q = self.questions.get_mut(&answer.question_id).unwrap();
        let p = 0.8;
        if answer.correct {
            q.probability = (1.0 as f64).min(q.probability * p + (1. - p));
        } else {
            q.probability = (0.0 as f64).max(q.probability * p);
        }
        q.answers.push(answer);
        q.probability
    }
}

pub struct QuestionService<'a> {
    repo: &'a db::Repository,
    prob_computer: ProbabilityComputer,
    questions: HashMap<String, HashMap<String, db::Question>>,
    sets: HashMap<String, Vec<(String, String)>>,
}

impl<'a> QuestionService<'a> {
    pub async fn new(repo: &db::Repository) -> Result<QuestionService> {
        let questionsdb = repo.get_all_questions().await?;
        let question_setsdb = repo.get_all_question_sets().await?;
        let answers = repo.get_all_answers().await?;

        let mut questions = HashMap::new();
        for q in &questionsdb {
            if !questions.contains_key(&q.factory) {
                questions.insert(q.factory.clone(), HashMap::new());
            }
            questions
                .get_mut(&q.factory)
                .unwrap()
                .insert(q.name.clone(), q.clone());
        }

        let mut sets = HashMap::new();
        for qs in question_setsdb {
            if !sets.contains_key(&qs.name) {
                sets.insert(qs.name.clone(), Vec::new());
            }
            let qq = repo.get_question_by_id(qs.id).await?;
            sets.get_mut(&qs.name).unwrap().push((qq.factory, qq.name));
        }

        Ok(QuestionService {
            repo,
            prob_computer: ProbabilityComputer::new(&answers, &questionsdb),
            questions,
            sets,
        })
    }

    pub async fn add_answer(&mut self, factory: &str, name: &str, correct: bool) -> Result<()> {
        let now = chrono::offset::Utc::now();
        let q = self
            .questions
            .get_mut(factory)
            .unwrap()
            .get_mut(name)
            .unwrap();
        let answer = db::Answer {
            id: 0,
            question_id: q.id,
            time: now,
            correct,
        };
        q.probability = self.prob_computer.add_answer(answer.clone());
        self.repo.add_answer(answer, q.probability).await?;
        Ok(())
    }

    pub fn get_random_selection(&'a self, set: &str, mut num: usize) -> Vec<&'a db::Question> {
        println!("set: {}, num: {}", set, num);
        let questions = self
            .sets
            .get(set)
            .unwrap()
            .iter()
            .map(|(f, n)| {
                println!("f: {}, n: {}", f, n);
                self.questions.get(f).unwrap().get(n).unwrap()
            })
            .collect::<Vec<&db::Question>>();
        let mut total = 0.;
        let mut stack = Vec::new();
        let mut chosen = HashSet::new();
        num = std::cmp::min(num, questions.len());
        // O(nk). Can be done in O(nlog(n)) using an augmented balanced search tree
        for _ in 0..num {
            for (idx, q) in questions.iter().enumerate() {
                if chosen.contains(&idx) {
                    continue;
                }
                total += 1. - q.probability;
                stack.push((idx, total));
            }
            let x = rand::random::<f64>() * total;
            for (name, v) in &stack {
                if *v >= x {
                    chosen.insert(*name);
                    break;
                }
            }
            stack.clear();
        }

        chosen
            .iter()
            .map(|&idx| questions.get(idx).unwrap().clone())
            .collect()
    }

    pub fn get_set_size(&self, name: &str) -> usize {
        self.sets.get(name).unwrap().len()
    }

    pub fn get_sets(&self) -> Vec<String> {
        self.sets.iter().map(|(name, _)| name.clone()).collect()
    }
}
