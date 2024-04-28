use crate::db;
use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use colored::Colorize;
use core::fmt;
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Text};
use num_format::{Locale, ToFormattedString};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::fs;
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

pub trait QuestionRunner {
    fn run(&self) -> Result<bool>;
    fn name(&self) -> String;
}

pub trait QuestionFactory {
    fn build(&self, data: &[u8]) -> Result<Box<dyn QuestionRunner>>;
}

pub trait QuestionSetFactory {
    fn build_set(&self, s: &Service, set_name: &str) -> Vec<QuestionID>;
    fn depends_on(&self) -> &Vec<String>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BaseQuestionSet {
    name: String,
    type_: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QuestionFactoryModel<T1: QuestionRunner, T2> {
    name: String,
    type_: String,
    items: Vec<T1>,
    data: T2,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QuestionSetFactoryModel<T> {
    name: String,
    type_: String,
    data: T,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NumericRangeData {
    question_prefix: String,
    range: f64,
    #[serde(skip)]
    depends: Vec<String>,
}

impl QuestionFactory for NumericRangeData {
    fn build(&self, data: &[u8]) -> Result<Box<dyn QuestionRunner>> {
        let mut question = serde_yaml::from_slice::<NumericRangeQuestion>(data)?;
        question.range = self.range;
        question.question = format!("{}{}?", self.question_prefix, question.question);
        Ok(Box::new(question) as Box<dyn QuestionRunner>)
    }
}

impl QuestionSetFactory for NumericRangeData {
    fn build_set(&self, s: &Service, set_name: &str) -> Vec<QuestionID> {
        s.get_factory(set_name).clone()
    }

    fn depends_on(&self) -> &Vec<String> {
        &self.depends
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

impl QuestionRunner for NumericRangeQuestion {
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
    #[serde(skip)]
    depends: Vec<String>,
}

impl QuestionFactory for DefaultData {
    fn build(&self, data: &[u8]) -> Result<Box<dyn QuestionRunner>> {
        let mut question = serde_yaml::from_slice::<DefaultQuestion>(data)?;
        question.question = format!("{}{}?", self.question_prefix, question.question);
        Ok(Box::new(question) as Box<dyn QuestionRunner>)
    }
}

impl QuestionSetFactory for DefaultData {
    fn build_set(&self, s: &Service, set_name: &str) -> Vec<QuestionID> {
        s.get_factory(set_name).clone()
    }

    fn depends_on(&self) -> &Vec<String> {
        &self.depends
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct DefaultQuestion {
    id: String,
    question: String,
    answers: Vec<String>,
}

impl QuestionRunner for DefaultQuestion {
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct UnionData {
    sets: Vec<String>,
}

impl QuestionSetFactory for UnionData {
    fn build_set(&self, s: &Service, _: &str) -> Vec<QuestionID> {
        let mut res = Vec::new();
        for set in &self.sets {
            res.extend_from_slice(&s.get_set(set));
        }
        res
    }

    fn depends_on(&self) -> &Vec<String> {
        &self.sets
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct Word {
    id: String,
    word: String,
    definition: String,
    example: String,
    translations: Vec<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct VocabData {
    #[serde(skip)]
    depends: Vec<String>,
}

impl QuestionRunner for Word {
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
    fn build(&self, data: &[u8]) -> Result<Box<dyn QuestionRunner>> {
        let question = serde_yaml::from_slice::<Word>(data)?;
        Ok(Box::new(question) as Box<dyn QuestionRunner>)
    }
}

impl QuestionSetFactory for VocabData {
    fn build_set(&self, s: &Service, set_name: &str) -> Vec<QuestionID> {
        s.get_factory(set_name).clone()
    }

    fn depends_on(&self) -> &Vec<String> {
        &self.depends
    }
}

fn pause_with_message(msg: &str) -> Result<()> {
    let mut stdout = stdout();
    stdout.write(msg.as_bytes())?;
    stdout.flush().unwrap();
    stdin().read(&mut [0])?;
    Ok(())
}

type QuestionID = i64;

pub struct Question {
    pub id: QuestionID,
    pub factory: String,
    pub name: String,
    pub probability: f64,
    pub num_correct: u32,
    pub num_incorrect: u32,
    pub runner: Box<dyn QuestionRunner>,
}

#[derive(Clone, Copy)]
pub enum Selection {
    All,
    Practiced,
}

impl fmt::Display for Selection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Selection::All => write!(f, "All"),
            Selection::Practiced => write!(f, "Practiced"),
        }
    }
}

pub struct Service<'a> {
    questions: HashMap<QuestionID, Question>,
    factories: HashMap<String, Vec<QuestionID>>,
    sets: HashMap<String, Vec<QuestionID>>,
    repo: &'a db::Repository,
    prob_computer: ProbabilityComputer,
}

impl<'a> Service<'a> {
    pub async fn new(repo: &db::Repository) -> Result<Service> {
        let questionsdb = repo.get_all_questions().await?;
        let factories = load_factories(&repo.get_all_question_factories().await?)?;
        let mut questions = HashMap::new();
        let mut by_factories = HashMap::new();
        for q in questionsdb {
            let factory = factories.get(&q.factory).unwrap();
            let runner = factory.build(&q.data)?;
            by_factories
                .entry(q.factory.clone())
                .or_insert(Vec::new())
                .push(q.id);
            questions.insert(
                q.id,
                Question {
                    id: q.id,
                    factory: q.factory,
                    name: q.name,
                    probability: q.probability,
                    num_correct: q.num_correct,
                    num_incorrect: q.num_incorrect,
                    runner,
                },
            );
        }

        let mut sets = HashMap::<String, Vec<QuestionID>>::new();
        let questions_in_set = repo.get_all_question_sets().await?;
        for qset in questions_in_set {
            let q = questions.get(&qset.question_id).unwrap();
            if !sets.contains_key(&qset.name) {
                sets.insert(qset.name.clone(), Vec::new());
            }
            sets.get_mut(&qset.name).unwrap().push(q.id);
        }

        let answers = repo
            .get_all_answers()
            .await?
            .iter()
            .map(|a| Answer {
                question_id: a.question_id,
                time: a.time,
                correct: a.correct,
            })
            .collect::<Vec<Answer>>();
        let prob_computer =
            ProbabilityComputer::new(answers, &questions.values().collect::<Vec<&Question>>());
        for &id in questions.keys() {
            repo.set_probability(id, prob_computer.get_prob(id)).await?;
        }

        Ok(Service {
            questions,
            sets,
            prob_computer,
            repo,
            factories: by_factories,
        })
    }

    pub async fn add_answer(&mut self, id: QuestionID, correct: bool) -> Result<()> {
        let now = chrono::offset::Utc::now();
        let q = self.questions.get_mut(&id).unwrap();
        q.probability = self.prob_computer.add_answer(Answer {
            question_id: q.id.clone(),
            time: now,
            correct,
        });
        self.repo
            .add_answer(q.id, now, correct, q.probability)
            .await?;
        Ok(())
    }

    fn filter_questions(
        &self,
        questions: &Vec<QuestionID>,
        selection: Selection,
    ) -> Vec<QuestionID> {
        match selection {
            Selection::All => questions.clone(),
            Selection::Practiced => questions
                .iter()
                .filter_map(|q| {
                    if self.prob_computer.questions.get(q).unwrap().answers.len() > 0 {
                        Some(*q)
                    } else {
                        None
                    }
                })
                .collect::<Vec<QuestionID>>(),
        }
    }

    pub fn get_weighted_random_selection(
        &self,
        set: &str,
        mut num: usize,
        selection: Selection,
    ) -> Vec<QuestionID> {
        let questions = self.filter_questions(self.sets.get(set).unwrap(), selection);
        let mut stack = Vec::new();
        let mut chosen = HashSet::new();
        num = std::cmp::min(num, questions.len());
        // O(nk). Can be done in O(nlog(n)) using an augmented balanced search tree
        for _ in 0..num {
            let mut total = 0.;
            for qid in questions.iter() {
                if chosen.contains(qid) {
                    continue;
                }
                let q = self.get(*qid);
                total += (1. - q.probability + 0.05).powf(1.5);
                stack.push((*qid, total));
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

        chosen.iter().map(|&qid| qid).collect::<Vec<QuestionID>>()
    }

    pub fn get_bottom_selection(
        &self,
        set: &str,
        num: usize,
        selection: Selection,
    ) -> Vec<QuestionID> {
        let mut question_ids = self.filter_questions(self.sets.get(set).unwrap(), selection);
        question_ids.sort_by(|&id1, &id2| {
            self.get(id1)
                .probability
                .total_cmp(&self.get(id2).probability)
        });
        question_ids[..num].to_vec()
    }

    pub fn get_uniform_random_selection(
        &self,
        set: &str,
        num: usize,
        selection: Selection,
    ) -> Vec<QuestionID> {
        let mut question_ids = self.filter_questions(self.sets.get(set).unwrap(), selection);
        question_ids.shuffle(&mut thread_rng());
        question_ids[..num].to_vec()
    }

    pub fn get_oldest_answer(
        &self,
        set: &str,
        num: usize,
        selection: Selection,
    ) -> Vec<QuestionID> {
        let question_ids = self.filter_questions(self.sets.get(set).unwrap(), selection);
        let mut times = Vec::new();
        for id in question_ids {
            let answers = self.prob_computer.get_answers(id);
            if let Some(a) = answers.last() {
                times.push((a.time, id));
            } else {
                times.push((DateTime::from_timestamp(0, 0).unwrap(), id));
            }
        }
        times.sort();
        times[..num].iter().map(|&(_, id)| id).collect()
    }

    pub fn get_set_size(&self, name: &str, selection: Selection) -> usize {
        let set = self.get_set(name);
        match selection {
            Selection::All => set.len(),
            Selection::Practiced => set
                .iter()
                .filter(|&q| self.prob_computer.questions.get(q).unwrap().answers.len() > 0)
                .count(),
        }
    }

    pub fn get_sets(&self) -> Vec<&String> {
        self.sets.keys().collect()
    }

    pub fn get(&self, id: QuestionID) -> &Question {
        self.questions.get(&id).unwrap()
    }

    pub fn last_answer(&self, id: QuestionID) -> Option<&Answer> {
        self.prob_computer.get_answers(id).last()
    }

    pub fn get_factory(&self, factory: &str) -> &Vec<QuestionID> {
        self.factories.get(factory).unwrap()
    }

    pub fn get_set(&self, set: &str) -> &Vec<QuestionID> {
        self.sets.get(set).unwrap()
    }

    pub async fn add_question_in_set(&mut self, id: QuestionID, set: &str) -> Result<bool> {
        let s = if let Some(s) = self.sets.get_mut(set) {
            s
        } else {
            self.sets.insert(String::from_str(set)?, Vec::new());
            self.sets.get_mut(set).unwrap()
        };

        // TODO Ass linear scan
        if s.contains(&id) {
            return Ok(false);
        }

        let q = self.questions.get(&id).unwrap();
        self.repo.insert_question_in_set(set, q.id).await?;
        s.push(id);
        Ok(true)
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

pub struct Answer {
    pub question_id: QuestionID,
    pub time: DateTime<Utc>,
    pub correct: bool,
}

struct ProbQuestion {
    answers: Vec<Answer>,
    weighted_total: f64,
    weighted_correct: f64,
}

struct ProbabilityComputer {
    questions: HashMap<QuestionID, ProbQuestion>,
}

impl ProbabilityComputer {
    fn new(answers: Vec<Answer>, questions: &[&Question]) -> ProbabilityComputer {
        let mut questions2 = HashMap::new();
        for q in questions {
            questions2.insert(
                q.id.clone(),
                ProbQuestion {
                    answers: Vec::new(),
                    weighted_total: 0.,
                    weighted_correct: 0.,
                },
            );
        }

        for a in answers {
            questions2.get_mut(&a.question_id).unwrap().answers.push(a);
        }

        for (_, q) in questions2.iter_mut() {
            q.answers.sort_by_key(|a| a.time);
            for c in q.answers.iter().map(|a| a.correct).collect::<Vec<bool>>() {
                ProbabilityComputer::add_to_question(q, c);
            }
        }

        ProbabilityComputer {
            questions: questions2,
        }
    }

    fn add_to_question(q: &mut ProbQuestion, correct: bool) {
        let p = 0.9;
        q.weighted_total = q.weighted_total * p + 1.;
        q.weighted_correct *= p;
        if correct {
            q.weighted_correct += 1.;
        }
    }

    fn add_answer(&mut self, answer: Answer) -> f64 {
        let q = self.questions.get_mut(&answer.question_id).unwrap();
        ProbabilityComputer::add_to_question(q, answer.correct);
        q.answers.push(answer);
        ProbabilityComputer::prob(q)
    }

    fn prob(q: &ProbQuestion) -> f64 {
        (q.weighted_correct + 1.) / (q.weighted_total + 2.)
    }

    fn get_prob(&self, id: QuestionID) -> f64 {
        ProbabilityComputer::prob(self.questions.get(&id).unwrap())
    }

    fn get_answers(&self, id: QuestionID) -> &Vec<Answer> {
        &self.questions.get(&id).unwrap().answers
    }
}

pub struct Models {
    pub questions: Vec<db::Question>,
    pub factories: Vec<db::QuestionFactory>,
    pub sets: HashMap<String, Box<dyn QuestionSetFactory>>,
}

pub fn load_models(paths: &[PathBuf]) -> Result<Models> {
    let mut models = Models {
        questions: Vec::new(),
        factories: Vec::new(),
        sets: HashMap::new(),
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
                models.sets.insert(
                    stuff.name.clone(),
                    Box::new(stuff.data.clone()) as Box<dyn QuestionSetFactory>,
                );
            }
            "numeric_range" => {
                let stuff = serde_yaml::from_slice::<
                    QuestionFactoryModel<NumericRangeQuestion, NumericRangeData>,
                >(&data)?;
                parse_factory::<NumericRangeQuestion, NumericRangeData>(&mut models, &stuff)?;
                models.sets.insert(
                    stuff.name.clone(),
                    Box::new(stuff.data.clone()) as Box<dyn QuestionSetFactory>,
                );
            }
            "vocab" => {
                let stuff = serde_yaml::from_slice::<QuestionFactoryModel<Word, VocabData>>(&data)?;
                parse_factory::<Word, VocabData>(&mut models, &stuff)?;
                models.sets.insert(
                    stuff.name.clone(),
                    Box::new(stuff.data.clone()) as Box<dyn QuestionSetFactory>,
                );
            }
            "union" => {
                let stuff = serde_yaml::from_slice::<QuestionSetFactoryModel<UnionData>>(&data)?;
                models.sets.insert(
                    stuff.name.clone(),
                    Box::new(stuff.data.clone()) as Box<dyn QuestionSetFactory>,
                );
            }
            _ => {
                panic!("unexpected question type {:?}", set.type_);
            }
        };
    }

    Ok(models)
}

fn parse_factory<T1, T2>(models: &mut Models, stuff: &QuestionFactoryModel<T1, T2>) -> Result<()>
where
    T1: Serialize + QuestionRunner,
    T2: Serialize,
{
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
