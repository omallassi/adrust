use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::NaiveTime;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::DateTime;
use tantivy::Index;
use tantivy::ReloadPolicy;

use std::path::Path;
use std::time::Instant;

extern crate slog;
extern crate slog_term;
use slog::*;

extern crate adr_config;
use adr_config::config::*;

extern crate adr_core;
use adr_core::adr_repo::*;

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

    slog::Logger::root(drain, o!())
}

pub fn build_index(index_path: String, adrs: Vec<Adr>) -> tantivy::Result<()> {
    info!(get_logger(), "Building Index in folder [{}]", index_path);

    let now = Instant::now();
    let index_path = Path::new(&index_path);

    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("status", TEXT | STORED);
    schema_builder.add_date_field("date", INDEXED | STORED);
    schema_builder.add_text_field("body", TEXT);
    schema_builder.add_text_field("tags", TEXT | STORED);
    schema_builder.add_text_field("path", TEXT | STORED);
    let schema = schema_builder.build();

    let mmap_directory = MmapDirectory::open(index_path)?;
    let index = Index::open_or_create(mmap_directory, schema.clone())?; // should use open_or_create to not overwrite existing index.
    let mut index_writer = index.writer(100_000_000)?; //multi threaded behind the scene # of thread < 8

    index_writer.delete_all_documents()?;
    index_writer.commit()?;

    let title = schema.get_field("title").unwrap();
    let status = schema.get_field("status").unwrap();
    let date = schema.get_field("date").unwrap();
    let body = schema.get_field("body").unwrap();
    let tags = schema.get_field("tags").unwrap();
    let path = schema.get_field("path").unwrap();

    for adr in adrs {
        //as usual, string / date conversions are a mess - All the following is to be able to index a datetime as expected by tantivy
        let adr_date_as_date = match NaiveDate::parse_from_str(&adr.date, "%Y-%m-%d") {
            Ok(r) => r,
            Err(why) => {
                debug!(
                    get_logger(),
                    "Pb while parsing date for ADR {:?} - {:?}",
                    adr.path(),
                    why
                );
                warn!(get_logger(), "Pb while parsing date for ADR {:?} - will use arbitraty January, 1rst 1970 date", adr.path().as_str());
                NaiveDate::from_ymd_opt(1970, 1, 1).unwrap()
            }
        };
        let zero_time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let date_time = NaiveDateTime::new(adr_date_as_date, zero_time);

        let epoc = date_time.and_utc().timestamp();

        index_writer
            .add_document(doc!(
                title => String::from(adr.title.as_str()),
                status => String::from(adr.status.as_str()),
                date => DateTime::from_timestamp_secs(epoc),
                body => String::from(adr.content.as_str()),
                tags => String::from(adr.tags.as_str()), //recreate a string from the tags Vec via Debug...
                path => String::from(adr.path().as_str()),
            ))
            .ok();
    }

    index_writer.commit()?;

    info!(
        get_logger(),
        "Indexing Time [{}] milli seconds",
        now.elapsed().as_millis()
    );

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SearchResult {
    pub title: [String; 1],
    pub status: [String; 1],
    pub date: [String; 1],
    pub tags: [String; 1],
    pub path: [String; 1],
}

pub fn search(
    index_path: String,
    query_as_string: String,
    limit: usize,
) -> tantivy::Result<Vec<SearchResult>> /*Result<()>*/
{
    debug!(
        get_logger(),
        "Searching [{}] based on Index in folder [{}]", query_as_string, index_path
    );

    let index_path = Path::new(&index_path);
    let mmap_directory = MmapDirectory::open(index_path)?;
    //println!("file exist {}", Index::exists(&mmap_directory) );
    let index = Index::open(mmap_directory)?;

    //
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("title", TEXT | STORED);
    schema_builder.add_text_field("status", TEXT | STORED);
    schema_builder.add_date_field("date", INDEXED | STORED);
    schema_builder.add_text_field("body", TEXT);
    schema_builder.add_text_field("tags", TEXT | STORED);
    schema_builder.add_text_field("path", TEXT | STORED);
    let schema = schema_builder.build();

    let title = schema.get_field("title").unwrap();
    let body = schema.get_field("body").unwrap();
    let status = schema.get_field("status").unwrap();
    let tags = schema.get_field("tags").unwrap();
    let path = schema.get_field("path").unwrap();

    //
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommit)
        .try_into()?;

    let searcher = reader.searcher();

    debug!(get_logger(), "Search with query [{}]", &query_as_string);

    // default_fields is the set of fields to use if none is specified in the query. date is not part of the default
    let query_parser = QueryParser::for_index(&index, vec![title, body, status, tags, path]);

    let query = match query_parser.parse_query(&query_as_string) {
        Ok(e) => e,
        Err(why) => panic!("Search | Error while parsing {:?}", why),
    };

    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

    let mut results = std::vec::Vec::new();
    for (_score, doc_address) in top_docs {
        let retrieved_doc = searcher.doc(doc_address)?;
        debug!(
            get_logger(),
            "Found doc [{}]",
            schema.to_json(&retrieved_doc)
        );

        let doc_as_json = schema.to_json(&retrieved_doc);
        let search_result: SearchResult = serde_json::from_str(&doc_as_json).unwrap();
        results.push(search_result);
    }

    Ok(results)
}
