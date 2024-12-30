use anyhow::{bail, Result};
use itertools::Itertools;
use log::{debug, info, warn};
use rayon::prelude::*;
use regex::Regex;
use rustc_hash::{FxHashMap, FxHashSet};
use std::fmt;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
mod cp437;

const SOURCES_FILE: &str = "shared/coha_sources.utf8.txt";
const LEXICON_FILE: &str = "shared/coha_lexicon.txt";
const CORPUS_DIR: &str = "db";
const CONTEXT: usize = 30;

enum Genre {
    Fic,
    Mag,
    News,
    Nf,
}

#[derive(Debug)]
struct TsvError {
    path: PathBuf,
    e: String,
}

impl std::error::Error for TsvError {}

impl fmt::Display for TsvError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.path.to_string_lossy(), self.e)
    }
}

fn tsv_err(path: &Path, e: &str) -> TsvError {
    TsvError {
        path: path.to_owned(),
        e: e.to_owned(),
    }
}

impl Genre {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "FIC" => Some(Genre::Fic),
            "MAG" => Some(Genre::Mag),
            "NEWS" => Some(Genre::News),
            "NF" => Some(Genre::Nf),
            _ => None,
        }
    }

    fn parse_for_files(path: &Path, s: &str) -> Result<Self> {
        match Genre::parse(s) {
            None => bail!(tsv_err(path, &format!("invalid genre: {s}"))),
            Some(x) => Ok(x),
        }
    }
}

impl fmt::Display for Genre {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Genre::Fic => "FIC",
                Genre::Mag => "MAG",
                Genre::News => "NEWS",
                Genre::Nf => "NF",
            }
        )
    }
}

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct TextId(usize);

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WordId(usize);

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct TokenId(usize);

#[derive(Copy, Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Year(u16);

struct Source {
    text_id: TextId,
    genre: Genre,
    year: Year,
    title: String,
    author: String,
}

pub struct Word {
    pub word_id: WordId,
    pub word_cs: String,
    pub word: String,
    pub lemma: String,
    pub pos: String,
}

struct Token {
    text_id: TextId,
    token_id: TokenId,
    word_id: WordId,
}

impl Source {
    fn parse_tsv(path: &Path, s: &str) -> Result<Self> {
        let mut fields = tsv_split(s);
        let mut next = || match fields.next() {
            None => Err(tsv_err(path, "TSV field missing")),
            Some(x) => Ok(x),
        };
        let text_id = TextId(next()?.parse()?);
        next()?; // # words
        let genre = Genre::parse_for_files(path, next()?)?;
        let year = Year(next()?.parse()?);
        let title = next()?.to_owned();
        let author = next()?.to_owned();
        Ok(Self {
            text_id,
            genre,
            year,
            title,
            author,
        })
    }
}

fn word_cleanup(x: &str) -> String {
    x.replace(|c: char| c.is_control(), "")
}

impl Word {
    fn parse_tsv(path: &Path, s: &str) -> Result<Self> {
        let mut fields = tsv_split(s);
        let mut next = || match fields.next() {
            None => Err(tsv_err(path, "TSV field missing")),
            Some(x) => Ok(x),
        };
        let word_id = WordId(next()?.parse()?);
        let word_cs = word_cleanup(next()?);
        let word = word_cleanup(next()?);
        let lemma = next()?.to_owned();
        let pos = next()?.to_owned();
        Ok(Self {
            word_id,
            word_cs,
            word,
            lemma,
            pos,
        })
    }
}

impl Token {
    fn parse_tsv(path: &Path, s: &str) -> Result<Self> {
        let mut fields = tsv_split(s);
        let mut next = || match fields.next() {
            None => Err(tsv_err(path, "TSV field missing")),
            Some(x) => Ok(x),
        };
        let text_id = TextId(next()?.parse()?);
        let token_id = TokenId(next()?.parse()?);
        let word_id = WordId(next()?.parse()?);
        Ok(Self {
            text_id,
            token_id,
            word_id,
        })
    }
}

fn tsv_split(s: &str) -> std::str::Split<char> {
    s.trim_end_matches(&['\n', '\r']).split('\t')
}

fn tsv_check_header<R: Read>(
    path: &Path,
    br: &mut BufReader<R>,
    exp_header: &[&str],
) -> Result<()> {
    let mut header = String::new();
    if br.read_line(&mut header)? == 0 {
        bail!(tsv_err(path, "header missing"));
    }
    let header: Vec<&str> = tsv_split(&header).collect();
    if header != exp_header {
        bail!(tsv_err(path, "unexpected headers"));
    }
    Ok(())
}

type Sources = FxHashMap<TextId, Source>;
type Lexicon = Vec<Option<Word>>;
type CohaFiles = Vec<CohaFile>;

pub struct Coha {
    sources: Sources,
    lexicon: Lexicon,
    coha_files: CohaFiles,
}

struct CohaFile {
    corpus_path: PathBuf,
    identifier: String,
}

pub enum CohaFilter {
    Any,
    Hash(FxHashSet<WordId>),
}

pub struct CohaSearch<'a> {
    pub label: String,
    pub filter_list: Vec<&'a CohaFilter>,
}

fn read_sources(root_dir: &Path) -> Result<Sources> {
    let path = root_dir.join(SOURCES_FILE);
    debug!("{}: reading...", path.to_string_lossy());
    let file = File::open(path.clone())?;
    let mut br = BufReader::new(file);

    let header = &[
        "textID",
        " # words ",
        "genre",
        "year",
        "title",
        "author",
        "Publication information",
        "Library of Congress classification (NF)",
        "FIXED",
    ];
    tsv_check_header(&path, &mut br, header)?;

    let mut sources = FxHashMap::default();
    let mut s = String::new();
    while br.read_line(&mut s)? > 0 {
        let source = Source::parse_tsv(&path, &s)?;
        sources.insert(source.text_id, source);
        s.clear();
    }
    info!("{}: {} sources", path.to_string_lossy(), sources.len());
    Ok(sources)
}

fn read_cp437_file_to_string(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut string = String::new();
    for b in bytes {
        string.push(cp437::CP437[b as usize]);
    }
    Ok(string)
}

fn read_lexicon(root_dir: &Path) -> Result<Lexicon> {
    let path = root_dir.join(LEXICON_FILE);
    debug!("{}: reading...", path.to_string_lossy());
    let file_string = read_cp437_file_to_string(&path)?;
    let mut br = BufReader::new(file_string.as_bytes());

    let header = &["wID", "wordCS", "word", "lemma", "PoS"];
    tsv_check_header(&path, &mut br, header)?;
    let separator = &["----", "----", "----", "----", "----"];
    tsv_check_header(&path, &mut br, separator)?;
    let empty = &[""];
    tsv_check_header(&path, &mut br, empty)?;

    let mut lexicon = Vec::new();
    let mut lexicon_padding: usize = 0;
    let mut s = String::new();
    while br.read_line(&mut s)? > 0 {
        let word = Word::parse_tsv(&path, &s)?;
        if word.word_id.0 < lexicon.len() {
            bail!(tsv_err(&path, "word IDs not increasing"));
        }
        while word.word_id.0 > lexicon.len() {
            lexicon_padding += 1;
            lexicon.push(None);
        }
        assert_eq!(word.word_id.0, lexicon.len());
        lexicon.push(Some(word));
        s.clear();
    }
    info!(
        "{}: {} words, {} padding",
        path.to_string_lossy(),
        lexicon.len() - lexicon_padding,
        lexicon_padding
    );
    Ok(lexicon)
}

fn read_corpus(root_dir: &Path) -> Result<CohaFiles> {
    let path = root_dir.join(CORPUS_DIR);
    debug!("{}: reading...", path.to_string_lossy());
    let mut corpus_paths = Vec::new();
    for subdir in path.read_dir()? {
        let subdir = subdir?.path();
        if subdir.is_dir() {
            for file in subdir.read_dir()? {
                let file = file?.path();
                let ext = file.extension();
                match ext {
                    None => continue,
                    Some(s) => {
                        if s != "txt" {
                            continue;
                        }
                    }
                };
                corpus_paths.push(file);
            }
        }
    }
    corpus_paths.sort();
    info!(
        "{}: {} corpus files",
        path.to_string_lossy(),
        corpus_paths.len()
    );
    corpus_paths.into_iter().map(CohaFile::new).collect()
}

impl Coha {
    pub fn load(root_dir: &Path) -> Result<Self> {
        let ((c, s), l) = rayon::join(
            || (read_corpus(root_dir), read_sources(root_dir)),
            || read_lexicon(root_dir),
        );
        let c = c?;
        let s = s?;
        let l = l?;
        Ok(Self {
            sources: s,
            lexicon: l,
            coha_files: c,
        })
    }

    pub fn get_filter<P>(&self, p: P) -> CohaFilter
    where
        P: Fn(&Word) -> bool,
    {
        CohaFilter::Hash(
            self.lexicon
                .iter()
                .filter_map(|w| match w {
                    None => None,
                    Some(w) => {
                        if p(w) {
                            Some(w.word_id)
                        } else {
                            None
                        }
                    }
                })
                .collect(),
        )
    }

    pub fn search(&self, result_dir: &Path, searches: &[&CohaSearch]) -> Result<()> {
        for search in searches {
            let filter_sizes = search
                .filter_list
                .iter()
                .map(|f| match f {
                    CohaFilter::Any => "∞".to_owned(),
                    CohaFilter::Hash(x) => x.len().to_string(),
                })
                .join(", ");
            info!("search {}: filter sizes: {}", search.label, filter_sizes);
            fs::create_dir_all(result_dir.join(&search.label))?;
        }
        let mut results = Vec::new();
        results.par_extend(
            self.coha_files
                .par_iter()
                .map(|cf| cf.search(self, result_dir, searches)),
        );
        for result in results {
            result?;
        }
        Ok(())
    }

    fn get_word(&self, word_id: WordId) -> &Word {
        match &self.lexicon[word_id.0] {
            Some(w) => w,
            None => panic!("expected valid word index"),
        }
    }

    fn get_text(&self, tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|t| &self.get_word(t.word_id).word_cs)
            .join(" ")
    }

    fn get_lemma_pos(&self, tokens: &[Token]) -> String {
        tokens
            .iter()
            .map(|t| {
                let word = self.get_word(t.word_id);
                format!("{}_{}", word.lemma, word.pos)
            })
            .join(" ")
    }
}

impl CohaFile {
    fn new(corpus_path: PathBuf) -> Result<Self> {
        let name = corpus_path
            .file_name()
            .expect("valid file name")
            .to_string_lossy()
            .into_owned();
        let re = Regex::new(r"^coha_db_(\d+s)\.txt$").unwrap();
        let identifier = match re.captures(&name) {
            None => bail!("unexpected file name {name}"),
            Some(caps) => caps.get(1).unwrap().as_str().to_owned(),
        };
        Ok(Self {
            corpus_path,
            identifier,
        })
    }
    fn search(&self, coha: &Coha, result_dir: &Path, searches: &[&CohaSearch]) -> Result<()> {
        let path = &self.corpus_path;
        debug!("{}: reading...", path.to_string_lossy());
        let mut writers = Vec::new();
        for search in searches {
            let outpath = result_dir.join(&search.label);
            let outpath = outpath.join(format!("{}-{}.csv", &search.label, &self.identifier));
            debug!("{}: writing...", outpath.to_string_lossy());
            let mut writer = csv::Writer::from_path(outpath)?;
            self.write_header(&mut writer, search.filter_list.len())?;
            writers.push(writer);
        }
        let file = File::open(path)?;
        let mut br = BufReader::new(file);
        let mut s = String::new();
        let mut tokens: Vec<Token> = Vec::new();
        let mut count_tokens: usize = 0;
        let mut count_texts: usize = 0;
        let mut total_hits: usize = 0;
        let mut hit_texts: usize = 0;

        let mut flush = |tokens: &mut Vec<Token>| -> Result<()> {
            let hits = self.search_text(coha, &mut writers, searches, tokens)?;
            total_hits += hits;
            if hits > 0 {
                hit_texts += 1;
            }
            count_texts += 1;
            tokens.clear();
            Ok(())
        };

        while br.read_line(&mut s)? > 0 {
            let token = Token::parse_tsv(path, &s)?;
            count_tokens += 1;
            if let Some(prev) = tokens.last() {
                if prev.text_id != token.text_id {
                    flush(&mut tokens)?;
                }
            }
            if let Some(prev) = tokens.last() {
                if prev.token_id >= token.token_id {
                    bail!(tsv_err(path, "token IDs not increasing"));
                }
            }
            tokens.push(token);
            s.clear();
        }
        if !tokens.is_empty() {
            flush(&mut tokens)?;
        }
        info!(
            "{}: {} tokens in {} texts, {} hits in {} texts",
            path.to_string_lossy(),
            count_tokens,
            count_texts,
            total_hits,
            hit_texts,
        );
        for mut writer in writers {
            writer.flush()?;
        }
        Ok(())
    }

    fn search_text(
        &self,
        coha: &Coha,
        writers: &mut [csv::Writer<File>],
        searches: &[&CohaSearch],
        tokens: &[Token],
    ) -> Result<usize> {
        assert!(!tokens.is_empty());
        assert!(tokens.first().unwrap().text_id == tokens.last().unwrap().text_id);
        let text_id = tokens.first().unwrap().text_id;
        let mut hits = 0;
        match coha.sources.get(&text_id) {
            None => warn!(
                "{}: uknown text ID {}",
                self.corpus_path.to_string_lossy(),
                text_id.0
            ),
            Some(source) => {
                for (writer, search) in writers.iter_mut().zip(searches) {
                    hits += self.search_text_one(coha, writer, search, source, tokens)?;
                }
            }
        }
        Ok(hits)
    }

    fn search_text_one(
        &self,
        coha: &Coha,
        writer: &mut csv::Writer<File>,
        search: &CohaSearch,
        source: &Source,
        tokens: &[Token],
    ) -> Result<usize> {
        let m = search.filter_list.len();
        let n = tokens.len();
        let mut hits = 0;
        if n >= m {
            'outer: for i in 0..(n - m + 1) {
                for j in 0..m {
                    let word_id = tokens[i + j].word_id;
                    if !match search.filter_list[j] {
                        CohaFilter::Any => true,
                        CohaFilter::Hash(x) => x.contains(&word_id),
                    } {
                        continue 'outer;
                    }
                }
                self.write_hit(coha, writer, source, tokens, i, m)?;
                hits += 1;
            }
        }
        Ok(hits)
    }

    fn write_header(&self, writer: &mut csv::Writer<File>, m: usize) -> Result<()> {
        let mut row = vec![
            "text ID".to_owned(),
            "genre".to_owned(),
            "year".to_owned(),
            "title".to_owned(),
            "author".to_owned(),
            "position".to_owned(),
        ];
        row.push("before".to_owned());
        for j in 0..m {
            row.push(format!("wordCS {}", j + 1));
        }
        row.push("after".to_owned());
        row.push("before_pos".to_owned());
        for j in 0..m {
            row.push(format!("word {}", j + 1));
            row.push(format!("lemma {}", j + 1));
            row.push(format!("pos {}", j + 1));
        }
        row.push("after_pos".to_owned());
        writer.write_record(row)?;
        Ok(())
    }

    fn write_hit(
        &self,
        coha: &Coha,
        writer: &mut csv::Writer<File>,
        source: &Source,
        tokens: &[Token],
        pos: usize,
        m: usize,
    ) -> Result<()> {
        let mut row = vec![
            source.text_id.0.to_string(),
            source.genre.to_string(),
            source.year.0.to_string(),
            source.title.to_owned(),
            source.author.to_owned(),
            pos.to_string(),
        ];
        let start = if pos < CONTEXT { 0 } else { pos - CONTEXT };
        let end = tokens.len().min(pos + m + CONTEXT);
        row.push(coha.get_text(&tokens[start..pos]));
        for j in 0..m {
            let word = coha.get_word(tokens[pos + j].word_id);
            row.push(word.word_cs.to_owned());
        }
        row.push(coha.get_text(&tokens[pos + m..end]));
        row.push(coha.get_lemma_pos(&tokens[start..pos]));
        for j in 0..m {
            let word = coha.get_word(tokens[pos + j].word_id);
            row.push(word.word.to_owned());
            row.push(word.lemma.to_owned());
            row.push(word.pos.to_owned());
        }
        row.push(coha.get_lemma_pos(&tokens[pos + m..end]));
        writer.write_record(row)?;
        Ok(())
    }
}
