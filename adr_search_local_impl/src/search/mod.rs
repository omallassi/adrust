use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::ReloadPolicy;
use tantivy::directory::MmapDirectory;

use std::path::Path;

extern crate slog;
extern crate slog_term;
use slog::*;

extern crate adr_config;
use adr_config::config::*;

fn get_logger() -> slog::Logger {
    let cfg: AdrToolConfig = get_config();

    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    let drain = slog::LevelFilter::new(
        drain,
        Level::from_usize(cfg.log_level).unwrap_or(Level::Debug),
    )
    .fuse();

    let log = slog::Logger::root(drain, o!());

    log
}

pub fn build_index(index_path: String) -> tantivy::Result<()>/*Result<(), ()>*/ {
    info!(get_logger(), "Building Index in folder [{}]", index_path);

    let index_path = Path::new(&index_path);

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT);
    let schema = schema_builder.build();

    let mmap_directory = MmapDirectory::open(index_path)?;
    let index = Index::create(mmap_directory, schema.clone())?; // should use open_or_create to not overwrite existing index.
    let mut index_writer = index.writer(100_000_000)?;

    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    index_writer.add_document(doc!(
    title => "Of Mice and Men",
    body => "A few miles south of Soledad, the Salinas River drops in close to the hillside \
                bank and runs deep and green. The water is warm too, for it has slipped twinkling \
                over the yellow sands in the sunlight before reaching the narrow pool. On one \
                side of the river the golden foothill slopes curve up to the strong and rocky \
                Gabilan Mountains, but on the valley side the water is lined with trees—willows \
                fresh and green with every spring, carrying in their lower leaf junctures the \
                debris of the winter’s flooding; and sycamores with mottled, white, recumbent \
                limbs and branches that arch over the pool"
    ));

    index_writer.add_document(doc!(
    title => "Of Mice and Men",
    body => "A few miles south of Soledad, the Salinas River drops in close to the hillside \
                bank and runs deep and green. The water is warm too, for it has slipped twinkling \
                over the yellow sands in the sunlight before reaching the narrow pool. On one \
                side of the river the golden foothill slopes curve up to the strong and rocky \
                Gabilan Mountains, but on the valley side the water is lined with trees—willows \
                fresh and green with every spring, carrying in their lower leaf junctures the \
                debris of the winter’s flooding; and sycamores with mottled, white, recumbent \
                limbs and branches that arch over the pool"
    ));

    index_writer.add_document(doc!(
    title => "Frankenstein",
    title => "The Modern Prometheus",
    body => "You will rejoice to hear that no disaster has accompanied the commencement of an \
                 enterprise which you have regarded with such evil forebodings.  I arrived here \
                 yesterday, and my first task is to assure my dear sister of my welfare and \
                 increasing confidence in the success of my undertaking."
    ));

    index_writer.commit()?;

    Ok(())
}

pub fn search(index_path: String, query_as_string: String) -> tantivy::Result<()>/*Result<()>*/ {
    debug!(get_logger(), "Searching [{}] based on Index in folder [{}]", query_as_string, index_path);

    let index_path = Path::new(&index_path);
    let mmap_directory = MmapDirectory::open(index_path)?;
    //println!("file exist {}", Index::exists(&mmap_directory) );
    let index = Index::open(mmap_directory)?;


    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("body", TEXT);
    let schema = schema_builder.build();
    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();

    //
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()?;

    let searcher = reader.searcher();

    let query_parser = QueryParser::for_index(&index, vec![title, body]);
    let query = query_parser.parse_query(&query_as_string)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;

    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address)?;
        println!("here {}", schema.to_json(&retrieved_doc));
    }

    println!("finish");
    Ok(())
}