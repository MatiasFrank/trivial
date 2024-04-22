use std::{collections::HashMap, fs};

use anyhow::Result;
use clap::Parser;
use rust::{
    db,
    functionality::{load_models, Service},
};

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

    let mut s = Service::new(&repo).await?;
    let edges: HashMap<&str, &Vec<String>> = models
        .sets
        .iter()
        .map(|(name, fac)| (name.as_str(), fac.depends_on()))
        .collect();
    let mut order = topsort(&edges);
    order.reverse();
    for set_name in order {
        let mut scount = 0;
        let factory = models.sets.get(set_name).unwrap();
        let questions = factory.build_set(&s, set_name);
        for q in questions {
            if s.add_question_in_set(q, set_name).await? {
                scount += 1;
            }
        }
        println!("Inserted {} questions into {:?}", scount, set_name);
    }

    Ok(())
}

fn topsort<'a>(edges: &'a HashMap<&'a str, &Vec<String>>) -> Vec<&'a str> {
    let mut in_degrees: HashMap<&str, usize> = edges.iter().map(|(node, _)| (*node, 0)).collect();
    for (_, es) in edges {
        for node2 in es.iter() {
            *in_degrees.get_mut(node2.as_str()).unwrap() += 1;
        }
    }

    let mut zeros = Vec::new();
    for (&node, &count) in &in_degrees {
        if count == 0 {
            zeros.push(node);
        }
    }

    let mut res = Vec::new();
    while !zeros.is_empty() {
        let node = zeros.pop().unwrap();
        res.push(node);
        for node2 in edges.get(node).unwrap().iter() {
            let deg = in_degrees.get_mut(node2.as_str()).unwrap();
            *deg -= 1;
            if *deg == 0 {
                res.push(node2.as_str());
            }
        }
    }

    res
}
