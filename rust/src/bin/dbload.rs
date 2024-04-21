use std::fs;

use anyhow::Result;
use clap::Parser;
use rust::{db, functionality::load_models};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the question set
    #[arg(short, long)]
    path: String,
    /// URL to the database
    #[arg(short, long)]
    db: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let url = format!("sqlite://{}", args.db);
    println!("url: {:?}", url);
    let repo = db::Repository::new(&url).await?;

    let mut paths = Vec::new();
    for path in fs::read_dir(args.path)? {
        paths.push(path?.path());
    }

    let models = load_models(&paths)?;
    let mut qcount = 0;
    for q in &models.questions {
        // TODO Fix this abstraction leaking
        if repo.has_question(&q.factory, &q.name).await? {
            continue;
        }
        repo.insert_question(&q.factory, &q.name, &q.data).await?;
        let qq = repo.get_question_by_name(&q.factory, &q.name).await?;
        repo.insert_question_in_set(&q.factory, qq.id).await?;
        qcount += 1;
    }

    let mut fcount = 0;
    for f in &models.factories {
        if repo.has_question_factory(&f.name).await? {
            continue;
        }
        repo.insert_question_factory(&f.name, &f.factory_type, &f.data)
            .await?;
        fcount += 1;
    }

    println!("Inserted {} questions and {} factories", qcount, fcount);

    Ok(())
}
