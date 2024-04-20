use anyhow::Result;
// use rand::prelude::*;
use rust::db;
use std::collections::{HashMap, HashSet};

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

struct QuestionService {
    repo: db::Repository,
    prob_computer: ProbabilityComputer,
    question_sets: HashMap<String, HashMap<String, db::Question>>,
}

impl QuestionService {
    async fn new(repo: db::Repository) -> Result<QuestionService> {
        let questions = repo.get_all_questions().await?;
        let answers = repo.get_all_answers().await?;
        let mut sets = HashMap::new();
        for q in &questions {
            if !sets.contains_key(&q.question_set) {
                sets.insert(q.question_set.clone(), HashMap::new());
            }
            sets.get_mut(&q.question_set)
                .unwrap()
                .insert(q.name.clone(), q.clone());
        }

        Ok(QuestionService {
            repo,
            prob_computer: ProbabilityComputer::new(&answers, &questions),
            question_sets: sets,
        })
    }

    fn get_random_selection(&self, set: &str, mut num: usize) -> Vec<db::Question> {
        let questions = self.question_sets.get(set).unwrap();
        let mut total = 0.;
        let mut stack = Vec::new();
        let mut chosen = HashSet::new();
        num = std::cmp::min(num, questions.len());
        // O(nk). Can be done in O(nlog(n)) using an augmented balanced search tree
        for _ in 0..num {
            for (_, q) in questions {
                if chosen.contains(&q.name) {
                    continue;
                }
                total += 1. - q.probability;
                stack.push((&q.name, total));
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
            .map(|&name| questions.get(name).unwrap().clone())
            .collect()
    }

    fn get_top_worst(&self, set: &str, mut num: usize) -> Vec<db::Question> {
        // let questions = self.question_sets.get(set).unwrap();

        Vec::new()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // let repo = Repository::new(DB_URL).await?;

    // let has = repo.has_question("some_set", "some_name").await?;
    // if !has {
    //     repo.insert_question("some_set", "some_name").await?;
    // }
    // let questions = repo.get_all_questions().await?;
    // println!("{}, {:?}", has, questions);

    // repo.add_answer("some_set", "some_name", 0.55, true).await?;
    // repo.add_answer("some_set", "some_name", 0.60, true).await?;
    // repo.add_answer("some_set", "some_name", 0.58, false)
    //     .await?;
    // println!("{:?}", repo.get_all_questions().await?);

    // println!("{:?}", repo.get_all_answers().await?);

    Ok(())
}
