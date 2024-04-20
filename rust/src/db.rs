use anyhow::Result;
use sqlx::{
    prelude::FromRow,
    types::chrono::{DateTime, Utc},
    Pool, Sqlite, SqlitePool,
};

// const DB_URL: &str = "sqlite://../sql/data.db";

#[derive(Clone, FromRow, Debug)]
pub struct Question {
    pub id: i64,
    pub question_set: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_answered_at: Option<DateTime<Utc>>,
    pub probability: f64,
    pub num_correct: u32,
    pub num_incorrect: u32,
    pub data: Vec<u8>,
}

#[derive(Clone, FromRow, Debug)]
pub struct Answer {
    pub id: i64,
    pub question_id: i64,
    pub time: DateTime<Utc>,
    pub correct: bool,
}

#[derive(Clone, FromRow, Debug)]
pub struct QuestionSet {
    pub id: i64,
    pub name: String,
    pub set_type: String,
    pub data: Vec<u8>,
}

pub struct Repository {
    db: Pool<Sqlite>,
}

impl Repository {
    pub async fn new(db_url: &str) -> Result<Repository> {
        let db = SqlitePool::connect(db_url).await?;
        Ok(Repository { db })
    }

    pub async fn get_all_questions(&self) -> Result<Vec<Question>> {
        let res = sqlx::query_as::<_, Question>("SELECT * FROM questions;")
            .fetch_all(&self.db)
            .await?;
        Ok(res)
    }

    pub async fn has_question(&self, question_set: &str, name: &str) -> Result<bool> {
        let res =
            sqlx::query("SELECT * FROM questions WHERE question_set = $1 AND name = $2 LIMIT 1")
                .bind(question_set)
                .bind(name)
                .fetch_optional(&self.db)
                .await?;
        Ok(res.is_some())
    }

    pub async fn get_question_by_name(&self, question_set: &str, name: &str) -> Result<Question> {
        let q = sqlx::query_as::<_, Question>(
            "
    SELECT * FROM questions WHERE  question_set = $1 AND name = $2 LIMIT 1;
            ",
        )
        .bind(question_set)
        .bind(name)
        .fetch_one(&self.db)
        .await?;
        Ok(q)
    }

    pub async fn get_question_by_id(&self, id: i64) -> Result<Question> {
        let q = sqlx::query_as::<_, Question>(
            "
    SELECT * FROM questions WHERE id = $1 LIMIT 1;
            ",
        )
        .bind(id)
        .fetch_one(&self.db)
        .await?;
        Ok(q)
    }

    pub async fn insert_question(
        &self,
        question_set: &str,
        name: &str,
        data: &Vec<u8>,
    ) -> Result<()> {
        let created_at = chrono::offset::Utc::now();
        let q = sqlx::query("INSERT INTO questions(question_set, name, created_at, probability, num_correct, num_incorrect, data) VALUES($1, $2, $3, $4, $5, $6, $7);")
            .bind(question_set)
            .bind(name)
            .bind(created_at)
            .bind(0)
            .bind(1)
            .bind(1)
            .bind(data);
        q.execute(&self.db).await?;
        Ok(())
    }

    pub async fn add_answer(&self, answer: Answer, new_prob: f64) -> Result<()> {
        let (cor, inc) = if answer.correct { (1, 0) } else { (0, 1) };
        sqlx::query(
            "
        UPDATE 
            questions
        SET
            probability = $1, 
            last_answered_at = $2,
            num_correct = num_correct + $3,
            num_incorrect = num_incorrect + $4
        WHERE
            id = $5
        ;",
        )
        .bind(new_prob)
        .bind(answer.time)
        .bind(cor)
        .bind(inc)
        .bind(answer.question_id)
        .execute(&self.db)
        .await?;

        sqlx::query(
            "
    INSERT INTO
            answers(question_id, time, correct)
            VALUES($1, $2, $3);",
        )
        .bind(answer.question_id)
        .bind(answer.time)
        .bind(answer.correct)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    pub async fn get_all_answers(&self) -> Result<Vec<Answer>> {
        let res = sqlx::query_as::<_, Answer>("SELECT * FROM answers;")
            .fetch_all(&self.db)
            .await?;
        Ok(res)
    }

    pub async fn has_question_set(&self, name: &str) -> Result<bool> {
        let res = sqlx::query("SELECT id FROM question_sets WHERE name = $1 LIMIT 1")
            .bind(name)
            .fetch_optional(&self.db)
            .await?;
        Ok(res.is_some())
    }

    pub async fn get_question_set(&self, name: &str) -> Result<QuestionSet> {
        let q = sqlx::query_as::<_, QuestionSet>(
            "
    SELECT * FROM question_set WHERE  name = $1 LIMIT 1;
            ",
        )
        .bind(name)
        .fetch_one(&self.db)
        .await?;
        Ok(q)
    }

    pub async fn insert_question_set(
        &self,
        name: &str,
        set_type: &str,
        data: &Vec<u8>,
    ) -> Result<()> {
        let q = sqlx::query("INSERT INTO question_sets(name, set_type, data) VALUES($1, $2, $3);")
            .bind(name)
            .bind(set_type)
            .bind(data);
        q.execute(&self.db).await?;
        Ok(())
    }

    pub async fn get_all_question_sets(&self) -> Result<Vec<QuestionSet>> {
        let res = sqlx::query_as::<_, QuestionSet>("SELECT * FROM question_set;")
            .fetch_all(&self.db)
            .await?;
        Ok(res)
    }
}
